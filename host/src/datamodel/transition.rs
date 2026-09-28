//! `TweenInfo` and `StyleRule:SetPropertyTransition` -- milestone 29 part C4.
//!
//! WHAT THIS SPRINT SHIPS. A cascade-driven property change (a `:Hover`
//! rule starting to match, a `StyleDerive` swap, anything `resolve` in
//! `cascade.rs` produces a NEW answer for) animates over a real `TweenInfo`
//! declared on the `StyleRule` responsible, instead of snapping the instant
//! the cascade re-resolves. Confirmed real against the milestone's own
//! investigation: `StyleRule:SetPropertyTransition` exists, takes a real
//! `TweenInfo`, and was previously confirmed unimplemented in this host.
//!
//! LINEAR EASING ONLY, THIS SPRINT. `TweenInfo` stores its own
//! `EasingStyle`/`EasingDirection` faithfully (`Enum.EasingStyle` has real
//! members this host already resolves generically through `enums.rs`), but
//! the actual interpolation curve `advance` applies is linear regardless of
//! which one a rule names. Building and testing the real bezier/bounce/
//! elastic curves for every `EasingStyle` member is a wider undertaking
//! than one sprint's own scope, and a linear curve is a complete, correct
//! animation on its own terms -- not a partial version of the others, the
//! same reasoning that let `ReducedMotionEnabled` (part C3) ship
//! recognized-but-unwired rather than guessed. Widening this to the real
//! curves is a stated later addition, not a bug in this file.
//!
//! ONLY WHAT THE CASCADE ITSELF CHANGES ANIMATES. A guest's own direct
//! write (`frame.BackgroundColor3 = x`) still snaps -- `Dom::styled_property`
//! only ever consults an active transition for a property that has NO
//! explicit value of its own, the same "instance's own explicit value
//! first" rule that already governs the cascade underneath it.
//!
//! WHERE THE RUNTIME STATE LIVES. `Dom::active_transitions` (currently
//! animating pairs) and `Dom::transitioned_targets` (the last cascade
//! answer seen for a pair that HAS a transition declared, the "did it
//! change" reference point and a fresh transition's own start value) are
//! both on `Dom` itself, not in this file -- the same reasoning
//! `style_properties`/`style_derives` already have there: this module
//! reads and writes them, it does not own the arena they live beside.

use mlua::prelude::*;
use mlua::{MetaMethod, UserData, UserDataFields, UserDataMethods};
use rbx_types::Variant;

use super::enums;
use super::{cascade, Dom};

/// One `TweenInfo` value, as `StyleRule:SetPropertyTransition` stores it.
/// `easing_style`/`easing_direction` are `Enum.EasingStyle`/
/// `Enum.EasingDirection` member numbers -- stored and read back faithfully
/// even though `advance`'s own interpolation does not consult them yet.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TweenInfoValue {
    pub time: f64,
    pub easing_style: u32,
    pub easing_direction: u32,
    pub repeat_count: i32,
    pub reverses: bool,
    pub delay_time: f64,
}

impl Default for TweenInfoValue {
    /// Real `TweenInfo.new()`'s own defaults: `Time = 1`,
    /// `EasingStyle = Quad`, `EasingDirection = Out`, `RepeatCount = 0`,
    /// `Reverses = false`, `DelayTime = 0`.
    fn default() -> Self {
        let quad = enums::item_by_name("EasingStyle", "Quad").map(|i| i.value);
        let out = enums::item_by_name("EasingDirection", "Out").map(|i| i.value);
        TweenInfoValue {
            time: 1.0,
            easing_style: quad.unwrap_or(0),
            easing_direction: out.unwrap_or(0),
            repeat_count: 0,
            reverses: false,
            delay_time: 0.0,
        }
    }
}

/// One `(instance, property)` pair currently animating.
#[derive(Clone, Debug)]
pub struct ActiveTransition {
    from: Variant,
    to: Variant,
    start: f64,
    duration: f64,
}

/// A `TweenInfo` value a guest holds.
#[derive(Clone, PartialEq)]
pub struct LuaTweenInfo(pub TweenInfoValue);

impl LuaTweenInfo {
    pub fn from_value(value: &LuaValue) -> Option<TweenInfoValue> {
        let LuaValue::UserData(ud) = value else {
            return None;
        };
        ud.borrow::<LuaTweenInfo>().ok().map(|t| t.0)
    }
}

impl UserData for LuaTweenInfo {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field(MetaMethod::Type, "TweenInfo");
        fields.add_field_method_get("Time", |_, this| Ok(this.0.time));
        fields.add_field_method_get("EasingStyle", |lua, this| {
            match enums::item_by_value("EasingStyle", this.0.easing_style) {
                Some(item) => item.into_lua(lua),
                None => Ok(LuaValue::Nil),
            }
        });
        fields.add_field_method_get("EasingDirection", |lua, this| {
            match enums::item_by_value("EasingDirection", this.0.easing_direction) {
                Some(item) => item.into_lua(lua),
                None => Ok(LuaValue::Nil),
            }
        });
        fields.add_field_method_get("RepeatCount", |_, this| Ok(this.0.repeat_count));
        fields.add_field_method_get("Reverses", |_, this| Ok(this.0.reverses));
        fields.add_field_method_get("DelayTime", |_, this| Ok(this.0.delay_time));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: LuaValue| {
            Ok(LuaTweenInfo::from_value(&other).as_ref() == Some(&this.0))
        });
    }
}

/// One positional `TweenInfo.new` argument, accepted as a member or as its
/// number, the same tolerance `content.rs`'s own `enum_argument` already
/// gives a `Font.new` caller.
fn enum_argument(value: &LuaValue, want: &str, what: &str) -> LuaResult<Option<u32>> {
    match value {
        LuaValue::Nil => Ok(None),
        LuaValue::UserData(ud) => {
            let item = ud
                .borrow::<enums::LuaEnumItem>()
                .map_err(|_| LuaError::runtime(format!("{what} expects an Enum.{want}")))?;
            if item.ty != want {
                return Err(LuaError::runtime(format!(
                    "{what} expects an Enum.{want}, got an Enum.{}",
                    item.ty
                )));
            }
            Ok(Some(item.value))
        }
        other => match super::number(other) {
            Some(n) => Ok(Some(super::whole_i32(n, what)? as u32)),
            None => Err(LuaError::runtime(format!(
                "{what} expects an Enum.{want}, got {}",
                other.type_name()
            ))),
        },
    }
}

pub fn install(lua: &Lua) -> LuaResult<()> {
    let tween_info = lua.create_table()?;
    tween_info.set(
        "new",
        lua.create_function(
            |_,
             (time, easing_style, easing_direction, repeat_count, reverses, delay_time): (
                LuaValue,
                LuaValue,
                LuaValue,
                LuaValue,
                LuaValue,
                LuaValue,
            )| {
                let mut info = TweenInfoValue::default();
                if let Some(n) = super::number(&time) {
                    info.time = n;
                }
                if let Some(raw) =
                    enum_argument(&easing_style, "EasingStyle", "TweenInfo.new's easingStyle")?
                {
                    info.easing_style = raw;
                }
                if let Some(raw) = enum_argument(
                    &easing_direction,
                    "EasingDirection",
                    "TweenInfo.new's easingDirection",
                )? {
                    info.easing_direction = raw;
                }
                if let Some(n) = super::number(&repeat_count) {
                    info.repeat_count = n as i32;
                }
                if let LuaValue::Boolean(b) = reverses {
                    info.reverses = b;
                }
                if let Some(n) = super::number(&delay_time) {
                    info.delay_time = n;
                }
                Ok(LuaTweenInfo(info))
            },
        )?,
    )?;
    lua.globals().set("TweenInfo", tween_info)?;
    Ok(())
}

/// Can `from` interpolate toward `to` at all? The same pair `lerp` would
/// accept -- checked once, up front, so a property type this sprint does
/// not know how to animate never gets an `ActiveTransition` created for it
/// in the first place and just snaps, exactly as it did before this file
/// existed.
fn interpolatable(from: &Variant, to: &Variant) -> bool {
    lerp(from, to, 0.0).is_some()
}

fn lerp_udim(a: rbx_types::UDim, b: rbx_types::UDim, t: f32) -> rbx_types::UDim {
    rbx_types::UDim::new(
        a.scale + (b.scale - a.scale) * t,
        (a.offset as f32 + (b.offset as f32 - a.offset as f32) * t).round() as i32,
    )
}

/// One interpolated step between `from` and `to` at `t` (0 at `from`, 1 at
/// `to`). `None` for a pair this sprint does not know how to animate --
/// the caller's own job to snap straight to `to` instead.
fn lerp(from: &Variant, to: &Variant, t: f32) -> Option<Variant> {
    match (from, to) {
        (Variant::Float32(a), Variant::Float32(b)) => Some(Variant::Float32(a + (b - a) * t)),
        (Variant::Float64(a), Variant::Float64(b)) => {
            Some(Variant::Float64(a + (b - a) * t as f64))
        }
        (Variant::Int32(a), Variant::Int32(b)) => Some(Variant::Int32(
            (*a as f32 + (*b as f32 - *a as f32) * t).round() as i32,
        )),
        (Variant::Int64(a), Variant::Int64(b)) => Some(Variant::Int64(
            (*a as f64 + (*b as f64 - *a as f64) * t as f64).round() as i64,
        )),
        (Variant::Color3(a), Variant::Color3(b)) => Some(Variant::Color3(rbx_types::Color3::new(
            a.r + (b.r - a.r) * t,
            a.g + (b.g - a.g) * t,
            a.b + (b.b - a.b) * t,
        ))),
        (Variant::UDim(a), Variant::UDim(b)) => Some(Variant::UDim(lerp_udim(*a, *b, t))),
        (Variant::UDim2(a), Variant::UDim2(b)) => Some(Variant::UDim2(rbx_types::UDim2::new(
            lerp_udim(a.x, b.x, t),
            lerp_udim(a.y, b.y, t),
        ))),
        (Variant::Vector2(a), Variant::Vector2(b)) => Some(Variant::Vector2(
            rbx_types::Vector2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t),
        )),
        _ => None,
    }
}

fn progress(active: &ActiveTransition, now: f64) -> f32 {
    if active.duration <= 0.0 {
        1.0
    } else {
        (((now - active.start) / active.duration) as f32).clamp(0.0, 1.0)
    }
}

/// The value an active transition on `key` currently shows, at `now`.
pub fn current(dom: &Dom, id: usize, key: &str, now: f64) -> Option<Variant> {
    let active = dom.active_transitions.get(&(id, key.to_string()))?;
    let t = progress(active, now);
    Some(lerp(&active.from, &active.to, t).unwrap_or_else(|| active.to.clone()))
}

/// Walks every instance at or under `root`, starts a new animation for any
/// `(instance, property)` pair whose cascade-resolved target changed since
/// last checked AND carries a declared transition, and drops any active
/// animation whose own duration has elapsed. Milestone 29 part C4 -- the
/// render loop's own per-frame driver, the same shape `apply_modifiers`
/// and `refresh_style_queries` already are.
pub fn advance(dom: &mut Dom, root: usize, now: f64) {
    let mut stack = vec![root];
    let mut changes: Vec<(usize, String, Variant, TweenInfoValue)> = Vec::new();
    while let Some(id) = stack.pop() {
        for (name, (value, rule)) in cascade::resolve_full(dom, id) {
            let Some(info) = dom.get_style_transition(rule, &name) else {
                continue;
            };
            let key = (id, name.clone());
            if dom.transitioned_targets.get(&key) != Some(&value) {
                changes.push((id, name, value, info));
            }
        }
        stack.extend(dom.children(id));
    }

    for (id, name, target, info) in changes {
        let key = (id, name);
        let from = current(dom, key.0, &key.1, now)
            .or_else(|| dom.transitioned_targets.get(&key).cloned())
            .unwrap_or_else(|| target.clone());
        dom.transitioned_targets.insert(key.clone(), target.clone());
        if interpolatable(&from, &target) {
            dom.active_transitions.insert(
                key,
                ActiveTransition {
                    from,
                    to: target,
                    start: now,
                    duration: info.time.max(0.0),
                },
            );
        } else {
            dom.active_transitions.remove(&key);
        }
    }

    let finished: Vec<_> = dom
        .active_transitions
        .iter()
        .filter(|(_, active)| now >= active.start + active.duration)
        .map(|(key, _)| key.clone())
        .collect();
    for key in finished {
        dom.active_transitions.remove(&key);
    }
}

/// Is anything animating right now? `main.rs`'s own render loop uses this
/// to keep repainting while a transition is in flight even though nothing
/// ELSE marked the tree dirty that frame -- a settled tree with no active
/// transition still costs nothing, the same `take_dirty`-gated idle path
/// every other frame already takes.
pub fn any_active(dom: &Dom) -> bool {
    !dom.active_transitions.is_empty()
}

impl Dom {
    /// Wires [`advance`] onto `Dom` itself, the same reason
    /// `apply_modifiers`/`refresh_style_queries` each got one in
    /// `cascade.rs`: `main.rs`'s own render loop lives outside
    /// `datamodel`'s own private module tree.
    ///
    /// TAKES `dt` AND ACCUMULATES `Dom::now` ITSELF, rather than reading a
    /// caller-supplied wall clock -- `services::Clock` (`desktop.Clock` on
    /// the guest side) is GATED ON HAVING A LISTENER SUBSCRIBED
    /// (`Clock::idle`), real and found live: a transition with nothing
    /// else in the mod subscribed to `Heartbeat` never ticks, `Now()`
    /// stays frozen at zero, and every `StyleRule` transition in the whole
    /// mod reads permanently at 0% progress. A `StyleRule` transition is
    /// an internal host mechanism, not a guest-visible signal a mod has to
    /// opt into for its own UI to animate, so it needs a time source that
    /// is not gated behind one. `dt` (the render loop's own per-frame
    /// delta, already computed whether or not anything is subscribed to
    /// `Clock`) is that source.
    pub fn advance_transitions(&mut self, root: usize, dt: f32) {
        self.now += dt as f64;
        let now = self.now;
        advance(self, root, now);
    }

    /// Wires [`any_active`] onto `Dom` itself, for the same reason.
    pub fn transitions_active(&self) -> bool {
        any_active(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_interpolation_at_the_midpoint_is_the_midpoint() {
        let got = lerp(&Variant::Float64(0.0), &Variant::Float64(10.0), 0.5);
        assert_eq!(got, Some(Variant::Float64(5.0)));
    }

    #[test]
    fn a_color3_interpolates_componentwise() {
        let a = Variant::Color3(rbx_types::Color3::new(0.0, 0.0, 0.0));
        let b = Variant::Color3(rbx_types::Color3::new(1.0, 1.0, 1.0));
        assert_eq!(
            lerp(&a, &b, 0.25),
            Some(Variant::Color3(rbx_types::Color3::new(0.25, 0.25, 0.25)))
        );
    }

    #[test]
    fn an_unsupported_pair_does_not_interpolate() {
        assert_eq!(lerp(&Variant::Bool(false), &Variant::Bool(true), 0.5), None);
        assert!(!interpolatable(&Variant::Bool(false), &Variant::Bool(true)));
    }

    #[test]
    fn tween_info_new_fills_in_real_defaults() {
        let info = TweenInfoValue::default();
        assert_eq!(info.time, 1.0);
        assert_eq!(info.repeat_count, 0);
        assert!(!info.reverses);
        assert_eq!(info.delay_time, 0.0);
    }
}
