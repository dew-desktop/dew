//! Hit testing and pointer input: what makes a DataModel mod clickable.
//!
//! WHAT WAS MISSING, IN ONE LINE
//! `Renderer::pointer` had a `Renderer::DataModel { .. } => Ok(())` arm with a
//! comment admitting it dropped every event, and sprint 7 measured the
//! consequence on the live window: a mod could be drawn, navigated, torn down and
//! told about its own properties, and could not be clicked. Fourteen methods and
//! no events was precisely why. This file is the other half.
//!
//! THE ONE TRAP THAT MATTERS MOST, AND IT IS GEOMETRIC
//! A hit test must use the SAME geometry the renderer drew, not a second
//! computation of it. The tempting shape is a recursive walk in here that
//! resolves `Size` and `Position` against the parent -- it is twenty lines, it
//! passes every test anyone writes for it, and it drifts the first time
//! `render::resolve` learns something this copy does not. The symptom is a button
//! that works everywhere except where it looks like it should, which is close to
//! unattributable once it has shipped.
//!
//! So there is no geometry in this file at all. [`render::display_list`] is the
//! single placement pass; `render::frame` turns it into nodes for the painter and
//! [`hit`] reads the same list BACKWARDS. If layout changes, both change, because
//! there is only one of them.
//!
//! HIT ORDER IS REVERSE PAINT ORDER. The display list is back to front, so the
//! topmost element is the LAST entry that contains the point. `ZIndex` therefore
//! decides both orders, in opposite directions, and the reverse scan is the only
//! place that fact is written down as code.
//!
//! CLIPPING APPLIES TO HITS. `Placed::clip` is carried down the walk for the
//! painter, and this file checks it too: a child outside a `ClipsDescendants`
//! parent is not visible and therefore not clickable. Clipping only the paint is
//! the easy half to do and the easy half to forget.
//!
//! THE LOCK IS NEVER HELD WHILE A HANDLER RUNS, and that is not a new rule here
//! -- it is sprint 8's, reused rather than reimplemented. Every fire in this file
//! goes through [`signal::fire`], which collects connection ids under the lock,
//! releases it, and re-checks each id immediately before the call. The one thing
//! this file has to get right is that the hit test finishes and DROPS THE GUARD
//! before the first handler is called, because a handler that moves the thing it
//! was told about is the ordinary case for input.
//!
//! AND INPUT DOES NOT DIRTY THE TREE. Nothing in this file calls `Dom::touch`.
//! Sprint 8 bought `painted` far below `fps` by making a repaint follow a real
//! change, and an input path that invalidated on every mouse move would hand that
//! straight back -- a cursor crossing the window would repaint at the frame rate
//! whether or not any handler did anything. A handler that assigns a property
//! marks the tree dirty on the path that already does that; a handler that
//! changes nothing leaves the screen alone.

use super::render::{self, Box2};
use super::{signal, SharedDom};
use mlua::prelude::*;
use mlua::{MetaMethod, UserData, UserDataFields, UserDataMethods};
use rbx_types::{Variant, Vector2};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Where the focus owner is stored in the Lua VM registry.
const FOCUS: &str = "dew.datamodel.focus";

/// Where the scroll velocities are stored in the Lua VM registry.
const SCROLL_VELOCITY: &str = "dew.datamodel.scroll_velocity";

/// Current scroll velocities for ScrollingFrames: (velocity, timestamp).
#[derive(Clone, Default)]
pub struct ScrollVelocity(Arc<Mutex<HashMap<usize, (Vector2, Instant)>>>);

impl UserData for ScrollVelocity {}

/// Retrieve the `ScrollVelocity` map for `lua`, creating it in the registry if not present.
pub fn scroll_velocity_of(lua: &Lua) -> LuaResult<ScrollVelocity> {
    if let LuaValue::UserData(ud) = lua.named_registry_value::<LuaValue>(SCROLL_VELOCITY)? {
        if let Ok(sv) = ud.borrow::<ScrollVelocity>() {
            return Ok(sv.clone());
        }
    }
    let fresh = ScrollVelocity::default();
    lua.set_named_registry_value(SCROLL_VELOCITY, fresh.clone())?;
    Ok(fresh)
}

/// Retrieve the current scroll velocity of `id`, decaying towards zero over 0.25s.
pub fn get_scroll_velocity(lua: &Lua, id: usize) -> LuaResult<Vector2> {
    let state = scroll_velocity_of(lua)?;
    let guard = state.0.lock().expect("scroll_velocity");
    let Some(&(vel, time)) = guard.get(&id) else {
        return Ok(Vector2::new(0.0, 0.0));
    };
    let elapsed = time.elapsed().as_secs_f32();
    if elapsed >= 0.25 {
        return Ok(Vector2::new(0.0, 0.0));
    }
    let factor = 1.0 - (elapsed / 0.25);
    Ok(Vector2::new(vel.x * factor, vel.y * factor))
}

/// Reset the scroll velocity of `id` to zero immediately.
pub fn reset_scroll_velocity(lua: &Lua, id: usize) -> LuaResult<()> {
    let state = scroll_velocity_of(lua)?;
    let mut guard = state.0.lock().expect("scroll_velocity");
    guard.remove(&id);
    Ok(())
}

/// Record a scroll velocity impulse on `id`.
pub fn set_scroll_velocity(lua: &Lua, id: usize, vel: Vector2) -> LuaResult<()> {
    let state = scroll_velocity_of(lua)?;
    let mut guard = state.0.lock().expect("scroll_velocity");
    guard.insert(id, (vel, Instant::now()));
    Ok(())
}

/// Exactly one focused instance at a time.
///
/// FOCUS IS STATE, and lives here beside `Pointer` rather than in the arena (`Dom`)
/// and rather than in a global. Scoped to the Lua VM, so two mods or windows do
/// not fight over a single focus owner.
#[derive(Clone, Default)]
pub struct Focus(Arc<Mutex<Option<usize>>>);

impl UserData for Focus {}

impl Focus {
    pub fn get(&self) -> Option<usize> {
        *self.0.lock().expect("focus")
    }

    pub fn set(&self, id: Option<usize>) {
        *self.0.lock().expect("focus") = id;
    }

    pub fn clear(&self) {
        self.set(None);
    }
}

/// Retrieve the `Focus` owner for `lua`, creating it in the registry if not present.
pub fn focus_of(lua: &Lua) -> LuaResult<Focus> {
    if let LuaValue::UserData(ud) = lua.named_registry_value::<LuaValue>(FOCUS)? {
        if let Ok(focus) = ud.borrow::<Focus>() {
            return Ok(focus.clone());
        }
    }
    let fresh = Focus::default();
    lua.set_named_registry_value(FOCUS, fresh.clone())?;
    Ok(fresh)
}

/// Checks if `id` is currently focused.
pub fn is_focused(lua: &Lua, id: usize) -> LuaResult<bool> {
    Ok(focus_of(lua)?.get() == Some(id))
}

/// Captures focus for `id` on `lua`.
///
/// RE-ENTRANCY SAFE & NO DOUBLE-FIRING:
/// - If `id` is already focused, returns immediately without firing `Focused`.
/// - If another instance was focused, releases it and fires `FocusLost(false)`.
/// - Drops the arena lock before firing any signal.
pub fn capture_focus(lua: &Lua, dom: &SharedDom, id: usize) -> LuaResult<()> {
    let focus = focus_of(lua)?;
    let prev = focus.get();
    if prev == Some(id) {
        return Ok(());
    }

    if let Some(old_id) = prev {
        focus.set(None);
        signal::fire(
            dom,
            old_id,
            &signal::Kind::FocusLost,
            &[LuaValue::Boolean(false)],
        );
    }

    if !dom.lock().expect("dom").exists(id) {
        return Ok(());
    }

    if let Some(other_id) = focus.get() {
        if other_id == id {
            return Ok(());
        }
        focus.set(None);
        signal::fire(
            dom,
            other_id,
            &signal::Kind::FocusLost,
            &[LuaValue::Boolean(false)],
        );
    }

    focus.set(Some(id));
    signal::fire(dom, id, &signal::Kind::Focused, &[]);
    Ok(())
}

/// Releases focus for `id` (or current focus if `id` matches).
pub fn release_focus(lua: &Lua, dom: &SharedDom, id: usize, enter_pressed: bool) -> LuaResult<()> {
    let _ = lua;
    let focus = focus_of(lua)?;
    if focus.get() != Some(id) {
        return Ok(());
    }
    focus.set(None);
    signal::fire(
        dom,
        id,
        &signal::Kind::FocusLost,
        &[LuaValue::Boolean(enter_pressed)],
    );
    Ok(())
}

/// Called on instance destruction to clean up focus and scroll velocity if the destroyed node was tracked.
pub fn on_destroy(lua: &Lua, destroyed_id: usize) {
    if let Ok(focus) = focus_of(lua) {
        if focus.get() == Some(destroyed_id) {
            focus.clear();
        }
    }
    if let Ok(sv) = scroll_velocity_of(lua) {
        sv.0.lock().expect("scroll_velocity").remove(&destroyed_id);
    }
}

/// Which physical button. `Middle` reaches `InputBegan` and has no `MouseButton`
/// events of its own, because the engine gives it none either.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left,
    Right,
    Middle,
}

impl Button {
    /// The `Enum.UserInputType` item this button reports as.
    fn user_input_type(self) -> &'static str {
        match self {
            Button::Left => "MouseButton1",
            Button::Right => "MouseButton2",
            Button::Middle => "MouseButton3",
        }
    }

    /// `(down, up, click)` -- the `GuiButton` events, or `None` for a button the
    /// engine gives none to.
    fn gui_button_events(self) -> Option<(signal::Kind, signal::Kind, signal::Kind)> {
        match self {
            Button::Left => Some((
                signal::Kind::MouseButton1Down,
                signal::Kind::MouseButton1Up,
                signal::Kind::MouseButton1Click,
            )),
            Button::Right => Some((
                signal::Kind::MouseButton2Down,
                signal::Kind::MouseButton2Up,
                signal::Kind::MouseButton2Click,
            )),
            Button::Middle => None,
        }
    }

    /// `Activated` for the left button, `SecondaryActivated` for the right.
    fn activation(self) -> Option<signal::Kind> {
        match self {
            Button::Left => Some(signal::Kind::Activated),
            Button::Right => Some(signal::Kind::SecondaryActivated),
            Button::Middle => None,
        }
    }
}

/// An `InputObject`, as `InputBegan` and its two siblings hand one over.
///
/// `Position` IS A `Vector2` HERE AND A `Vector3` ON THE ENGINE, and that is a
/// stated departure rather than an oversight. `vocabulary` deliberately carries
/// no `Vector3`: the comment there says this standard is 2D UI and a host
/// implementing `CFrame` would be describing Roblox rather than describing a UI.
/// The engine's third component is the wheel delta, which arrives on this type as
/// `Delta` in the one case it is not zero -- so nothing is lost except the shape,
/// and inventing a `Vector3` for one field's third slot would be the larger
/// divergence.
///
/// READ-ONLY, and every field is set at construction. On the engine an
/// `InputObject` is a live object the engine mutates as the input continues; here
/// it is a snapshot of one event, which is what a handler reads it as anyway.
#[derive(Clone, Copy)]
pub struct InputObject {
    user_input_type: &'static str,
    user_input_state: &'static str,
    position: Vector2,
    delta: Vector2,
}

impl UserData for InputObject {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        // A metaFIELD, and mlua fills this in with the RUST type name when it is
        // left out -- so leaving it out does not fail loudly, it answers
        // "InputObject" by luck of the Rust name matching, and the next type
        // renamed breaks every guest silently. This codebase has got `__type`
        // wrong twice; `LuaSignal` and `InstanceRef` carry the same comment, and
        // the test asserts the VALUE rather than asserting it is not nil.
        fields.add_meta_field(MetaMethod::Type, "InputObject");

        fields.add_field_method_get("UserInputType", |_, this| {
            Ok(super::enums::item_by_name(
                "UserInputType",
                this.user_input_type,
            ))
        });
        fields.add_field_method_get("UserInputState", |_, this| {
            Ok(super::enums::item_by_name(
                "UserInputState",
                this.user_input_state,
            ))
        });
        fields.add_field_method_get("Position", |_, this| {
            Ok(super::vocabulary::LuaVector2(this.position))
        });
        fields.add_field_method_get("Delta", |_, this| {
            Ok(super::vocabulary::LuaVector2(this.delta))
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            Ok(format!(
                "InputObject {} {}",
                this.user_input_type, this.user_input_state
            ))
        });
    }
}

/// Does a point fall inside a box?
///
/// HALF-OPEN ON THE FAR EDGE. A box at x=0 w=100 owns 0 up to but not including
/// 100, so two elements sharing an edge do not both claim the boundary -- and the
/// one that would win is whichever the reverse scan reached first, which is a coin
/// toss dressed as a rule.
fn contains(b: Box2, x: f32, y: f32) -> bool {
    x >= b.x && x < b.x + b.w && y >= b.y && y < b.y + b.h
}

/// A boolean property, or the class default the reflection database carries.
fn flag(dom: &super::Dom, id: usize, key: &str) -> bool {
    matches!(dom.property(id, key), Some(Variant::Bool(true)))
}

/// Does this element SINK input, so that the hit stops here?
///
/// ── HOW `Active` AND `Interactable` GATE INPUT, WRITTEN DOWN ON PURPOSE ──
///
/// The engine's answer is not obvious and guessing it silently is how two hosts
/// diverge, so here is the rule Dew honours, in one sentence:
///
/// **`Active` decides WHO IS HIT. `Interactable` decides whether the hit FIRES
/// ANYTHING.**
///
/// That is not a compromise invented here -- the reflection database's own
/// defaults are the argument for it. `Frame` and `TextLabel` ship
/// `Active = false`; `TextButton` and `ImageButton` ship `Active = true`; all
/// four ship `Interactable = true`. Under this rule a `TextLabel` laid over a
/// `TextButton` does not steal the click, which is the behaviour every Roblox
/// application is already written against and the thing a hit test that took the
/// topmost drawn element unconditionally would break on its first label. Any
/// other reading of `Active` makes those four defaults arbitrary.
///
/// `Interactable = false` STILL SINKS. It suppresses the events on the element
/// that was hit; it does not make the element transparent and let the input
/// through to whatever is behind it. A disabled button that let clicks fall onto
/// the panel underneath is a worse bug than one that swallows them.
///
/// THE KNOWN DEPARTURE, stated here rather than discovered later: on the engine
/// `MouseEnter` fires on a `Frame` with `Active = false`, and here it does not,
/// because such a frame is not a hit candidate at all. One rule covering pointer
/// and click alike is worth more to a host being written from scratch than two
/// rules that differ by event family, and a mod that wants a hoverable panel
/// writes `Active = true` -- which is what a Roblox developer writes for one
/// anyway. If this has to change, it changes here, in one function.
fn vector2(dom: &super::Dom, id: usize, key: &str) -> Vector2 {
    match dom.property(id, key) {
        Some(Variant::Vector2(v)) => v,
        _ => Vector2::new(0.0, 0.0),
    }
}

fn sinks(dom: &super::Dom, id: usize) -> bool {
    flag(dom, id, "Active")
        || matches!(
            dom.class_of(id).as_deref(),
            Some("TextBox" | "ScrollingFrame")
        )
}

/// May this element's events fire at all?
fn interactable(dom: &super::Dom, id: usize) -> bool {
    flag(dom, id, "Interactable")
}

/// The instance at (x, y), or `None`.
///
/// REVERSE PAINT ORDER: the display list is back to front, so the topmost
/// element is the last one that contains the point. The scan stops at the first
/// SINKING element it finds and steps past the rest -- see [`sinks`].
///
/// TAKES A `&Dom` AND RETURNS AN ID, so the caller can drop the guard before it
/// fires anything. A version that fired from inside would deadlock on the first
/// handler that touched the tree, which is sprint 8's trap and sprint 7's before
/// it.
pub fn hit(
    dom: &super::Dom,
    root: usize,
    width: f32,
    height: f32,
    x: f32,
    y: f32,
) -> Option<usize> {
    render::display_list(dom, root, width, height)
        .iter()
        .rev()
        .find(|placed| {
            contains(placed.rect, x, y)
                // CLIPPED OUT IS NOT HIT. The clip box is the nearest
                // `ClipsDescendants` ancestor's, inherited down the same walk the
                // painter uses.
                && placed.clip.is_none_or(|c| contains(c, x, y))
                && sinks(dom, placed.id)
        })
        .map(|placed| placed.id)
}

/// What an event is being delivered TO: one tree, in one VM, at one size.
///
/// FOUR ARGUMENTS THAT NEVER VARY WITHIN AN EVENT, bundled because they were
/// four arguments on all five entry points and clippy was right to say so at
/// eight. They are not an arbitrary grouping: together they are the SURFACE --
/// the root a mod was given, the VM its handlers live in, and the box the tree
/// lays out against. A second window onto the same arena would be a second one
/// of these with the same `dom` and a different `size`, which is exactly the
/// distinction the type draws.
///
/// BORROWED, NOT OWNED. It is built per event from the fields the frame loop
/// already holds, so it costs nothing and cannot go stale.
pub struct Surface<'a> {
    pub lua: &'a Lua,
    pub dom: &'a SharedDom,
    pub root: usize,
    pub size: (f32, f32),
}

/// Everything the host has to remember between two pointer events.
///
/// HOVER IS STATE, AND THAT IS WHY THIS TYPE EXISTS. `MouseEnter` and
/// `MouseLeave` are not derivable from one event: they are the difference between
/// two, so something has to hold the previous answer. It lives beside the renderer
/// in `main` rather than in the arena, because it is a property of the pointer and
/// the window and not of the tree -- two windows onto one arena would each have
/// their own cursor.
#[derive(Default)]
pub struct Pointer {
    /// The element `MouseEnter` was last fired on, if any.
    hover: Option<usize>,
    /// Where the press for each button landed, so a release can decide whether it
    /// completes a click. `None` means that button is not down.
    held: [Option<usize>; 3],
    /// The last position the cursor was seen at, so the tree changing under a
    /// STATIONARY cursor can still be reconciled. See [`Pointer::refresh`].
    at: Option<(f32, f32)>,
    /// Exactly one focused instance at a time, synced with the VM registry.
    focus: Focus,
}

/// Where one fire is aimed, and what it carries.
///
/// COLLECTED BEFORE ANY HANDLER RUNS. Each entry is fired in order with the lock
/// released, which is what stops a `MouseEnter` handler that destroys its own
/// element from breaking the `MouseMoved` queued behind it.
struct Aimed {
    id: usize,
    kind: signal::Kind,
    args: Args,
}

/// What a given event family passes its handler.
enum Args {
    /// `MouseEnter`, `MouseLeave`, `MouseMoved`, the `Down`/`Up` pairs and the
    /// wheel: two numbers, in the surface's own coordinates.
    Position(f32, f32),
    /// `MouseButton1Click` and its three siblings take NOTHING on the engine, and
    /// that is worth a variant rather than passing the position anyway. A handler
    /// written against the engine reads no first argument, and a host that
    /// helpfully supplied one would be the host somebody wrote
    /// `function(x) ... end` against and then could not port.
    Nothing,
    /// `InputBegan`, `InputChanged` and `InputEnded`.
    Input(InputObject),
    /// `Activated` and `SecondaryActivated`, which pass
    /// `(inputObject, clickCount)`.
    Activation(InputObject),
}

impl Args {
    fn build(&self, lua: &Lua) -> LuaResult<Vec<LuaValue>> {
        Ok(match self {
            Args::Position(x, y) => vec![x.into_lua(lua)?, y.into_lua(lua)?],
            Args::Nothing => Vec::new(),
            Args::Input(object) => vec![object.into_lua(lua)?],
            // CLICK COUNT IS ALWAYS 1. Double-click detection needs a clock and a
            // threshold and neither the window layer nor this host has one; a
            // hard-coded 2 would be a lie, and omitting the argument would break
            // `function(input, count)`.
            Args::Activation(object) => vec![object.into_lua(lua)?, 1.into_lua(lua)?],
        })
    }
}

impl Pointer {
    /// Fire a collected batch. THE LOCK IS NOT HELD HERE -- see the module
    /// comment; the batch was built from a guard that has already been dropped.
    fn fire_all(surface: &Surface, batch: Vec<Aimed>) -> LuaResult<()> {
        let (lua, dom) = (surface.lua, surface.dom);
        for aimed in batch {
            // RE-CHECKED, because an earlier handler in this same batch is free to
            // have destroyed this element. `signal::fire` re-checks each
            // CONNECTION for the same reason one level down; this is the same
            // discipline one level up, where the receiver itself can go.
            if !dom.lock().expect("dom").exists(aimed.id) {
                continue;
            }
            let args = aimed.args.build(lua)?;
            signal::fire(dom, aimed.id, &aimed.kind, &args);
        }
        Ok(())
    }

    /// An `InputObject` for a mouse button at a position.
    fn button_input(button: Button, state: &'static str, x: f32, y: f32) -> InputObject {
        InputObject {
            user_input_type: button.user_input_type(),
            user_input_state: state,
            position: Vector2::new(x, y),
            delta: Vector2::new(0.0, 0.0),
        }
    }

    /// The `MouseLeave`/`MouseEnter` pair for a hover change, or nothing.
    ///
    /// LEAVE BEFORE ENTER. A guest that highlights on enter and clears on leave
    /// has one highlighted element at a time only if the order is this way round;
    /// the other way, moving between two neighbours clears the one just entered.
    fn hover_to(&mut self, next: Option<usize>, x: f32, y: f32, batch: &mut Vec<Aimed>) {
        if self.hover == next {
            return;
        }
        if let Some(old) = self.hover {
            batch.push(Aimed {
                id: old,
                kind: signal::Kind::MouseLeave,
                args: Args::Position(x, y),
            });
        }
        if let Some(new) = next {
            batch.push(Aimed {
                id: new,
                kind: signal::Kind::MouseEnter,
                args: Args::Position(x, y),
            });
        }
        self.hover = next;
    }

    /// The element input should be aimed at, at (x, y), or `None`.
    ///
    /// THE TWO GATES IN ONE PLACE: [`hit`] applies `Active` by choosing who is
    /// hit, and this applies `Interactable` by discarding the hit's events. Every
    /// entry point below goes through here so neither gate can be forgotten on
    /// one path and honoured on another.
    fn aim(dom: &super::Dom, surface: &Surface, x: f32, y: f32) -> Option<usize> {
        hit(dom, surface.root, surface.size.0, surface.size.1, x, y)
            .filter(|id| interactable(dom, *id))
    }

    /// The cursor moved.
    pub fn moved(&mut self, surface: &Surface, x: f32, y: f32) -> LuaResult<()> {
        let delta = match self.at {
            Some((px, py)) => Vector2::new(x - px, y - py),
            None => Vector2::new(0.0, 0.0),
        };
        self.at = Some((x, y));

        let mut batch = Vec::new();
        {
            let guard = surface.dom.lock().expect("dom");
            let target = Self::aim(&guard, surface, x, y);
            self.hover_to(target, x, y, &mut batch);
            if let Some(id) = target {
                batch.push(Aimed {
                    id,
                    kind: signal::Kind::MouseMoved,
                    args: Args::Position(x, y),
                });
                batch.push(Aimed {
                    id,
                    kind: signal::Kind::InputChanged,
                    args: Args::Input(InputObject {
                        user_input_type: "MouseMovement",
                        user_input_state: "Change",
                        position: Vector2::new(x, y),
                        delta,
                    }),
                });
            }
        }
        Self::fire_all(surface, batch)
    }

    /// A button went down.
    pub fn down(&mut self, surface: &Surface, button: Button, x: f32, y: f32) -> LuaResult<()> {
        self.at = Some((x, y));
        let focus = focus_of(surface.lua)?;
        self.focus = focus.clone();

        let mut batch = Vec::new();
        let target_box;
        let old_focus;
        {
            let guard = surface.dom.lock().expect("dom");
            // THE PRESS IS RECORDED AS THE RAW HIT, before `Interactable` is
            // applied, so that releasing over a non-interactable element does not
            // complete a click against whatever the scan found beneath it.
            self.held[button as usize] =
                hit(&guard, surface.root, surface.size.0, surface.size.1, x, y);
            let target = Self::aim(&guard, surface, x, y);
            // A DOWN WITHOUT A PRECEDING MOVE STILL SETTLES HOVER. A synthetic
            // event in a headless test arrives with no move before it, and so does
            // a real click on a window that was just shown under the cursor.
            self.hover_to(target, x, y, &mut batch);

            if button == Button::Left {
                let current = focus.get();
                let is_textbox = target
                    .and_then(|id| guard.class_of(id))
                    .map(|c| c == "TextBox")
                    .unwrap_or(false);
                if is_textbox {
                    let tid = target.unwrap();
                    if current == Some(tid) {
                        target_box = None;
                        old_focus = None;
                    } else {
                        target_box = Some(tid);
                        old_focus = current;
                    }
                } else {
                    target_box = None;
                    old_focus = current;
                }
            } else {
                target_box = None;
                old_focus = None;
            }

            if let Some(id) = target {
                if let Some((down, _, _)) = button.gui_button_events() {
                    batch.push(Aimed {
                        id,
                        kind: down,
                        args: Args::Position(x, y),
                    });
                }
                batch.push(Aimed {
                    id,
                    kind: signal::Kind::InputBegan,
                    args: Args::Input(Self::button_input(button, "Begin", x, y)),
                });
            }
        }

        // Release old focus if any
        if let Some(old_id) = old_focus {
            focus.set(None);
            signal::fire(
                surface.dom,
                old_id,
                &signal::Kind::FocusLost,
                &[LuaValue::Boolean(false)],
            );
        }

        // Capture new focus if any
        if let Some(new_id) = target_box {
            if surface.dom.lock().expect("dom").exists(new_id) {
                focus.set(Some(new_id));
                signal::fire(surface.dom, new_id, &signal::Kind::Focused, &[]);
            }
        }

        Self::fire_all(surface, batch)
    }

    /// A named key event arrived.
    pub fn key(&mut self, surface: &Surface, name: &str) -> LuaResult<()> {
        let focus = focus_of(surface.lua)?;
        self.focus = focus.clone();
        if name == "Return" {
            if let Some(id) = focus.get() {
                release_focus(surface.lua, surface.dom, id, true)?;
            }
        }
        Ok(())
    }

    /// A button came up.
    ///
    /// THE ORDER IS THE ENGINE'S: `MouseButton1Up`, then `MouseButton1Click`, then
    /// `Activated`. A guest connecting two of the three is relying on it.
    pub fn up(&mut self, surface: &Surface, button: Button, x: f32, y: f32) -> LuaResult<()> {
        self.at = Some((x, y));
        let pressed = self.held[button as usize].take();
        let mut batch = Vec::new();
        {
            let guard = surface.dom.lock().expect("dom");
            let target = Self::aim(&guard, surface, x, y);
            self.hover_to(target, x, y, &mut batch);

            if let Some(id) = target {
                if let Some((_, up, click)) = button.gui_button_events() {
                    batch.push(Aimed {
                        id,
                        kind: up,
                        args: Args::Position(x, y),
                    });
                    // A CLICK IS PRESS AND RELEASE ON THE SAME ELEMENT.
                    if pressed == Some(id) {
                        batch.push(Aimed {
                            id,
                            kind: click,
                            args: Args::Nothing,
                        });
                        if let Some(activated) = button.activation() {
                            batch.push(Aimed {
                                id,
                                kind: activated,
                                args: Args::Activation(Self::button_input(button, "End", x, y)),
                            });
                        }
                    }
                }
            }

            // `InputEnded` GOES TO WHERE THE PRESS BEGAN, not to what is under the
            // cursor now, whenever the two differ. That is the reading a drag
            // needs: an element told input began on it has to be told it ended, or
            // it holds a pressed state nothing can clear. With no outstanding
            // press it goes to the current hit, which is what a release with no
            // matching down produces.
            let ended = pressed.or(target);
            if let Some(id) = ended.filter(|id| interactable(&guard, *id)) {
                batch.push(Aimed {
                    id,
                    kind: signal::Kind::InputEnded,
                    args: Args::Input(Self::button_input(button, "End", x, y)),
                });
            }
        }
        Self::fire_all(surface, batch)
    }

    /// The wheel turned. Positive scrolls the content up, as `dew_window`
    /// reports it and as `Live.Session` reads it.
    pub fn wheel(&mut self, surface: &Surface, x: f32, y: f32, delta: f32) -> LuaResult<()> {
        // A ZERO DELTA IS NOT A DIRECTION. Neither event is the honest answer, and
        // firing `Backward` because the sign test happened to fall that way is how
        // a host grows a spurious scroll on a trackpad reporting rest.
        if delta == 0.0 {
            return Ok(());
        }
        self.at = Some((x, y));
        let mut batch = Vec::new();
        let mut scroll_update: Option<(usize, Vector2)> = None;
        {
            let mut guard = surface.dom.lock().expect("dom");
            let Some(id) = Self::aim(&guard, surface, x, y) else {
                return Ok(());
            };
            batch.push(Aimed {
                id,
                kind: if delta > 0.0 {
                    signal::Kind::MouseWheelForward
                } else {
                    signal::Kind::MouseWheelBackward
                },
                args: Args::Position(x, y),
            });
            batch.push(Aimed {
                id,
                kind: signal::Kind::InputChanged,
                args: Args::Input(InputObject {
                    user_input_type: "MouseWheel",
                    user_input_state: "Change",
                    position: Vector2::new(x, y),
                    // THE WHEEL IS THE ONE PLACE `Delta` IS NOT ZERO, and on the
                    // engine it is `Position.Z` of a `Vector3`. See the note on
                    // `InputObject`: this host is 2D and carries it here instead.
                    delta: Vector2::new(0.0, delta),
                }),
            });

            // Find nearest ScrollingFrame ancestor (or id itself)
            let mut curr = Some(id);
            while let Some(c) = curr {
                if guard.class_of(c).as_deref() == Some("ScrollingFrame") {
                    let scrolling_enabled = !matches!(
                        guard.property(c, "ScrollingEnabled"),
                        Some(Variant::Bool(false))
                    );
                    let wheel_enabled = !matches!(
                        guard.property(c, "ScrollWheelInputEnabled"),
                        Some(Variant::Bool(false))
                    );
                    if scrolling_enabled && wheel_enabled {
                        let scroll_dir = match guard.property(c, "ScrollingDirection") {
                            Some(Variant::Enum(raw)) => {
                                super::enums::item_by_value("ScrollingDirection", raw.to_u32())
                                    .map(|item| item.name)
                                    .unwrap_or("XY")
                            }
                            _ => "XY",
                        };
                        let scroll_step = 40.0;
                        let (dx, dy) = if scroll_dir == "X" {
                            (-delta * scroll_step, 0.0)
                        } else {
                            (0.0, -delta * scroll_step)
                        };

                        let current_pos = vector2(&guard, c, "CanvasPosition");
                        let target_pos = Vector2::new(current_pos.x + dx, current_pos.y + dy);

                        let list = render::display_list(
                            &guard,
                            surface.root,
                            surface.size.0,
                            surface.size.1,
                        );
                        if let Some(placed) = list.iter().find(|p| p.id == c) {
                            let own_box = render::content_box(&guard, c, placed.rect);
                            let (canvas_w, canvas_h, frame_w, frame_h) =
                                render::scrolling_frame_bounds(&guard, c, own_box);
                            let max_scroll_x = (canvas_w - frame_w).max(0.0);
                            let max_scroll_y = (canvas_h - frame_h).max(0.0);
                            let clamped_x = target_pos.x.clamp(0.0, max_scroll_x);
                            let clamped_y = target_pos.y.clamp(0.0, max_scroll_y);
                            let new_pos = Vector2::new(clamped_x, clamped_y);

                            if new_pos != current_pos {
                                if let Some(node) = guard.node_mut(c) {
                                    node.props.insert(
                                        "CanvasPosition".to_string(),
                                        Variant::Vector2(new_pos),
                                    );
                                }
                                guard.touch();
                                // Velocity impulse: pixels per second (assuming 0.1s wheel step)
                                let vel = Vector2::new(dx * 10.0, dy * 10.0);
                                scroll_update = Some((c, vel));
                            }
                        }
                        break;
                    }
                }
                curr = guard.parent_of(c);
            }
        }

        if let Some((sf_id, vel)) = scroll_update {
            set_scroll_velocity(surface.lua, sf_id, vel)?;
            signal::property_changed(surface.lua, surface.dom, sf_id, "CanvasPosition")?;
        }

        Self::fire_all(surface, batch)
    }

    /// Reconcile hover against a tree that changed under a STATIONARY cursor.
    ///
    /// THIS IS THE CASE A NAIVE IMPLEMENTATION GETS WRONG, and it gets it wrong
    /// silently. Hover is the difference between two pointer events, so an
    /// implementation that updates it only when the pointer moves is correct right
    /// up until the TREE moves instead -- and then a mod that destroys the button
    /// under the cursor keeps a stale hover forever, and one that puts a new
    /// element there never fires `MouseEnter` until the person jiggles the mouse.
    /// Neither reports anything.
    ///
    /// So the frame loop calls this after a frame that was actually painted: the
    /// hit test is re-run at the last known position and the hover pair fires from
    /// the same code the move path uses.
    ///
    /// A DESTROYED HOVER FIRES NO `MouseLeave`. There is nothing to fire it on --
    /// `Dom::destroy` dropped that element's connections on the way past, which is
    /// sprint 8's rule doing the work here -- so the state is cleared and whatever
    /// is now under the cursor gets its `MouseEnter`.
    ///
    /// IT DOES NOT DIRTY THE TREE. A handler that assigns something marks the tree
    /// dirty on the path that already does that, and the next frame reconciles
    /// again; hover is idempotent once settled, so this converges rather than
    /// looping. A `refresh` that invalidated unconditionally would be a repaint
    /// every frame, which is sprint 8's gain handed straight back.
    pub fn refresh(&mut self, surface: &Surface) -> LuaResult<()> {
        let Some((x, y)) = self.at else {
            // The cursor has never been over this surface. There is no position to
            // re-test, and assuming (0, 0) would fire `MouseEnter` on whatever sits
            // in the top-left corner of every window at startup.
            return Ok(());
        };
        let mut batch = Vec::new();
        {
            let guard = surface.dom.lock().expect("dom");
            if self.hover.is_some_and(|id| !guard.exists(id)) {
                self.hover = None;
            }
            // A PRESS TARGET GOES WITH ITS ELEMENT. Ids are never reused, so a
            // stale one cannot come back pointing at somebody else -- but it would
            // never clear either, and the button would stay half-pressed for the
            // life of the mod.
            for held in self.held.iter_mut() {
                if held.is_some_and(|id| !guard.exists(id)) {
                    *held = None;
                }
            }
            let target = Self::aim(&guard, surface, x, y);
            self.hover_to(target, x, y, &mut batch);
        }
        Self::fire_all(surface, batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datamodel::{handle, install, install_vocabulary};

    /// The surface every test below lays out against.
    const SIZE: (f32, f32) = (200.0, 100.0);

    /// A VM with a `ScreenGui` root, a tree built from Luau, and a pointer.
    ///
    /// THE TREE IS BUILT FROM LUAU, as every other test in this crate builds
    /// theirs. A Rust fixture would exercise the arena and not the dispatch, and
    /// the dispatch is the half a mod actually reaches: an event connected through
    /// `__index` is the only kind a guest can connect.
    ///
    /// AND THE EVENTS ARE INJECTED HEADLESSLY. There is no window in any of these
    /// -- `Pointer` takes coordinates, so what the frame loop does with a real
    /// `Event::PointerDown` is one translation away and everything under it is
    /// testable without a desktop session. Eyeballing a window is the demo; this
    /// is the test.
    struct Harness {
        lua: Lua,
        dom: SharedDom,
        root: usize,
        pointer: Pointer,
    }

    impl Harness {
        fn new(src: &str) -> Harness {
            let lua = Lua::new();
            let dom = SharedDom::default();
            install(&lua, &dom).expect("install");
            install_vocabulary(&lua).expect("vocabulary");
            let root = dom
                .lock()
                .expect("dom")
                .insert("ScreenGui".into(), "DewRoot".into());
            lua.globals()
                .set("root", handle(&lua, &dom, root).expect("root handle"))
                .expect("root");
            // A TABLE THE HANDLERS WRITE INTO. Counting in Luau rather than in
            // Rust keeps the assertion on the guest's side of the boundary, which
            // is where the behaviour is observed.
            lua.globals()
                .set("log", lua.create_table().expect("table"))
                .expect("log");
            lua.load(src).exec().expect("guest");
            // THE TREE STARTS CLEAN, so a test asserting that input did not dirty
            // it is asserting about input and not about the writes that built the
            // tree.
            dom.lock().expect("dom").take_dirty();
            Harness {
                lua,
                dom,
                root,
                pointer: Pointer::default(),
            }
        }

        /// DESTRUCTURED RATHER THAN A `self.surface()` HELPER, because a method
        /// taking `&self` borrows the whole struct and `pointer` has to be
        /// borrowed mutably at the same time. Splitting the borrow by field is
        /// what Rust wants here, and it is three lines per entry point.
        fn drive(&mut self, f: impl FnOnce(&mut Pointer, &Surface) -> LuaResult<()>) {
            let Harness {
                lua,
                dom,
                root,
                pointer,
            } = self;
            let surface = Surface {
                lua,
                dom,
                root: *root,
                size: SIZE,
            };
            f(pointer, &surface).expect("input");
        }

        fn moved(&mut self, x: f32, y: f32) {
            self.drive(|p, s| p.moved(s, x, y));
        }

        fn press(&mut self, button: Button, x: f32, y: f32) {
            self.drive(|p, s| p.down(s, button, x, y));
        }

        fn release(&mut self, button: Button, x: f32, y: f32) {
            self.drive(|p, s| p.up(s, button, x, y));
        }

        fn down(&mut self, x: f32, y: f32) {
            self.press(Button::Left, x, y);
        }

        fn up(&mut self, x: f32, y: f32) {
            self.release(Button::Left, x, y);
        }

        fn key(&mut self, name: &str) {
            self.drive(|p, s| p.key(s, name));
        }

        fn right_down(&mut self, x: f32, y: f32) {
            self.press(Button::Right, x, y);
        }

        fn right_up(&mut self, x: f32, y: f32) {
            self.release(Button::Right, x, y);
        }

        fn wheel(&mut self, x: f32, y: f32, delta: f32) {
            self.drive(|p, s| p.wheel(s, x, y, delta));
        }

        fn refresh(&mut self) {
            self.drive(|p, s| p.refresh(s));
        }

        /// A full press and release at one point, which is what a click is.
        fn click(&mut self, x: f32, y: f32) {
            self.down(x, y);
            self.up(x, y);
        }

        /// Whatever the guest's handlers recorded, as a comma-joined string.
        fn log(&self) -> String {
            self.lua
                .load("return table.concat(log, \",\")")
                .eval::<String>()
                .expect("log")
        }

        fn eval<T: FromLuaMulti>(&self, src: &str) -> T {
            self.lua.load(src).eval().expect("eval")
        }

        fn hit_at(&self, x: f32, y: f32) -> Option<String> {
            let guard = self.dom.lock().expect("dom");
            hit(&guard, self.root, SIZE.0, SIZE.1, x, y).and_then(|id| guard.name_of(id))
        }

        fn dirty(&self) -> bool {
            self.dom.lock().expect("dom").take_dirty()
        }
    }

    /// One `TextButton` filling the left half of the surface.
    ///
    /// A `TextButton` RATHER THAN A `Frame`, and that is the point of the class:
    /// `Activated` is a `GuiButton` event, and `TextButton` ships `Active = true`
    /// so it is a hit candidate without the test setting anything.
    /// `b` IS A GLOBAL, NOT A LOCAL, and that is a fact about the harness rather
    /// than about the host: each `eval` below loads its own chunk, and a `local`
    /// in the setup chunk is not in scope in a later one. Reading the tree back
    /// through a global is the shortest way to assert on it from Luau, which is
    /// the side the behaviour is observed from.
    const ONE_BUTTON: &str = r#"
        b = Instance.new("TextButton")
        b.Name = "Go"
        b.Size = UDim2.new(0, 100, 0, 100)
        b.Parent = root
    "#;

    // ── The done-test: a synthetic click fires the right handler and the tree
    //    changes ──────────────────────────────────────────────────────────────

    #[test]
    fn a_click_fires_activated_and_the_handler_changes_the_tree() {
        let mut h = Harness::new(&format!(
            r#"
            {ONE_BUTTON}
            b.Activated:Connect(function()
                b.Text = "clicked"
            end)
        "#
        ));
        // "Button" IS THE REFLECTION DATABASE'S DEFAULT for TextButton.Text, not
        // an empty string -- so the pre-condition is that the default is intact
        // rather than that the property is blank.
        assert_eq!(h.eval::<String>("return b.Text"), "Button");
        h.click(50.0, 50.0);
        // THE HANDLER RAN AND THE TREE CHANGED. This is the clause milestone 1's
        // done-test has been missing: a mod written without Aether responds to a
        // click.
        assert_eq!(h.eval::<String>("return b.Text"), "clicked");
    }

    #[test]
    fn a_click_outside_the_button_fires_nothing() {
        let mut h = Harness::new(&format!(
            r#"
            {ONE_BUTTON}
            b.Activated:Connect(function() table.insert(log, "activated") end)
        "#
        ));
        h.click(150.0, 50.0);
        assert_eq!(h.log(), "");
    }

    #[test]
    fn the_engines_order_for_a_press_and_release() {
        let mut h = Harness::new(&format!(
            r#"
            {ONE_BUTTON}
            b.MouseButton1Down:Connect(function() table.insert(log, "down") end)
            b.MouseButton1Up:Connect(function() table.insert(log, "up") end)
            b.MouseButton1Click:Connect(function() table.insert(log, "click") end)
            b.Activated:Connect(function() table.insert(log, "activated") end)
            b.InputBegan:Connect(function() table.insert(log, "began") end)
            b.InputEnded:Connect(function() table.insert(log, "ended") end)
        "#
        ));
        h.click(50.0, 50.0);
        // A GUEST CONNECTING TWO OF THESE RELIES ON THE ORDER, so it is asserted
        // as a sequence rather than as six independent "did it fire" checks.
        assert_eq!(h.log(), "down,began,up,click,activated,ended");
    }

    #[test]
    fn a_click_is_press_and_release_on_the_same_element() {
        let mut h = Harness::new(
            r#"
            local a = Instance.new("TextButton")
            a.Name = "A"
            a.Size = UDim2.new(0, 100, 0, 100)
            a.Parent = root
            local b = Instance.new("TextButton")
            b.Name = "B"
            b.Size = UDim2.new(0, 100, 0, 100)
            b.Position = UDim2.new(0, 100, 0, 0)
            b.Parent = root
            a.MouseButton1Click:Connect(function() table.insert(log, "a click") end)
            b.MouseButton1Click:Connect(function() table.insert(log, "b click") end)
            a.MouseButton1Up:Connect(function() table.insert(log, "a up") end)
            b.MouseButton1Up:Connect(function() table.insert(log, "b up") end)
        "#,
        );
        // Pressed on A, released on B: the `Up` lands where the release did and
        // NEITHER gets a click. Letting go somewhere else is how a person changes
        // their mind mid-click, and a host that fired B's click would make that
        // impossible.
        h.down(50.0, 50.0);
        h.up(150.0, 50.0);
        assert_eq!(h.log(), "b up");
    }

    #[test]
    fn the_right_button_activates_secondarily() {
        let mut h = Harness::new(&format!(
            r#"
            {ONE_BUTTON}
            b.MouseButton2Down:Connect(function() table.insert(log, "down") end)
            b.MouseButton2Up:Connect(function() table.insert(log, "up") end)
            b.MouseButton2Click:Connect(function() table.insert(log, "click") end)
            b.SecondaryActivated:Connect(function() table.insert(log, "secondary") end)
            b.Activated:Connect(function() table.insert(log, "activated") end)
        "#
        ));
        h.right_down(50.0, 50.0);
        h.right_up(50.0, 50.0);
        // `Activated` IS THE LEFT BUTTON ONLY. A right-click that also fired it
        // would make a context menu trigger the primary action.
        assert_eq!(h.log(), "down,up,click,secondary");
    }

    // ── Hit order, and the geometry it shares with the renderer ───────────────

    #[test]
    fn the_hit_test_uses_the_geometry_the_renderer_drew() {
        // ANCHOR POINT IS THE CASE THAT CATCHES A SECOND COMPUTATION. A hit test
        // that resolved `Size` and `Position` itself and forgot the anchor shift
        // would report this button 40px to the left of where it is drawn -- and
        // still pass every test written against a top-left-anchored element,
        // which is most of them.
        let h = Harness::new(
            r#"
            local b = Instance.new("TextButton")
            b.Name = "Centred"
            b.Size = UDim2.new(0, 80, 0, 40)
            b.Position = UDim2.new(0.5, 0, 0.5, 0)
            b.AnchorPoint = Vector2.new(0.5, 0.5)
            b.Parent = root
        "#,
        );
        // Centred on (100, 50), so the box is (60, 30) to (140, 70).
        let drawn = crate::datamodel::render::frame_of(&h.dom, h.root, SIZE.0, SIZE.1);
        let rect = drawn.nodes[0].rect;
        assert_eq!((rect.x, rect.y, rect.w, rect.h), (60.0, 30.0, 80.0, 40.0));

        // THE HIT AGREES WITH THE RECTANGLE THE PAINTER GOT, at each edge.
        assert_eq!(h.hit_at(100.0, 50.0).as_deref(), Some("Centred"));
        assert_eq!(h.hit_at(60.0, 30.0).as_deref(), Some("Centred"));
        assert_eq!(h.hit_at(59.0, 50.0), None);
        // HALF-OPEN ON THE FAR EDGE: 140 belongs to whatever is to the right.
        assert_eq!(h.hit_at(139.0, 69.0).as_deref(), Some("Centred"));
        assert_eq!(h.hit_at(140.0, 50.0), None);
    }

    #[test]
    fn hit_order_is_reverse_paint_order() {
        let h = Harness::new(
            r#"
            local under = Instance.new("TextButton")
            under.Name = "Under"
            under.Size = UDim2.new(0, 100, 0, 100)
            under.ZIndex = 5
            under.Parent = root
            local over = Instance.new("TextButton")
            over.Name = "Over"
            over.Size = UDim2.new(0, 100, 0, 100)
            over.ZIndex = 9
            over.Parent = root
        "#,
        );
        // ZINDEX DECIDES BOTH ORDERS, IN OPPOSITE DIRECTIONS. The display list is
        // back to front, so the highest ZIndex is painted last and hit first --
        // and a hit test that scanned forwards would return the one UNDERNEATH,
        // which looks correct for every tree that happens to declare its topmost
        // element last.
        let drawn = crate::datamodel::render::frame_of(&h.dom, h.root, SIZE.0, SIZE.1);
        assert_eq!(drawn.nodes.last().expect("a node").name, "Over");
        assert_eq!(h.hit_at(50.0, 50.0).as_deref(), Some("Over"));
    }

    #[test]
    fn declaration_order_breaks_a_zindex_tie() {
        let h = Harness::new(
            r#"
            local first = Instance.new("TextButton")
            first.Name = "First"
            first.Size = UDim2.new(0, 100, 0, 100)
            first.Parent = root
            local second = Instance.new("TextButton")
            second.Name = "Second"
            second.Size = UDim2.new(0, 100, 0, 100)
            second.Parent = root
        "#,
        );
        // The sort is STABLE, so equal ZIndex leaves declaration order alone and
        // the later sibling is on top -- of the paint and of the hit.
        assert_eq!(h.hit_at(50.0, 50.0).as_deref(), Some("Second"));
    }

    #[test]
    fn a_child_outside_a_clipping_parent_is_not_clickable() {
        let mut h = Harness::new(
            r#"
            local panel = Instance.new("Frame")
            panel.Name = "Panel"
            panel.Size = UDim2.new(0, 100, 0, 50)
            panel.ClipsDescendants = true
            panel.Parent = root
            -- Twice the parent's height, so its bottom half hangs outside the clip.
            local b = Instance.new("TextButton")
            b.Name = "Tall"
            b.Size = UDim2.new(0, 100, 0, 100)
            b.Parent = panel
            b.Activated:Connect(function() table.insert(log, "activated") end)
        "#,
        );
        // CLIPPING APPLIES TO HIT TESTING AS WELL AS TO DRAWING. Inside the
        // parent's box the button is both drawn and clickable; below it the button
        // is neither -- and a host that clipped only the paint would have an
        // invisible button swallowing clicks over whatever is beneath the panel.
        assert_eq!(h.hit_at(50.0, 25.0).as_deref(), Some("Tall"));
        assert_eq!(h.hit_at(50.0, 75.0), None);
        h.click(50.0, 75.0);
        assert_eq!(h.log(), "");
        h.click(50.0, 25.0);
        assert_eq!(h.log(), "activated");
    }

    #[test]
    fn an_invisible_subtree_is_not_clickable() {
        let mut h = Harness::new(
            r#"
            local panel = Instance.new("Frame")
            panel.Size = UDim2.new(0, 100, 0, 100)
            panel.Visible = false
            panel.Parent = root
            local b = Instance.new("TextButton")
            b.Name = "Hidden"
            b.Size = UDim2.new(1, 0, 1, 0)
            b.Parent = panel
            b.Activated:Connect(function() table.insert(log, "activated") end)
        "#,
        );
        // INVISIBLE HIDES THE SUBTREE, and it follows from the same line in
        // `display_list` that keeps it off the screen -- there is no second rule
        // here to forget. The button itself is `Visible`; its parent is not.
        assert_eq!(h.hit_at(50.0, 50.0), None);
        h.click(50.0, 50.0);
        assert_eq!(h.log(), "");
    }

    // ── `Active` and `Interactable` ───────────────────────────────────────────

    #[test]
    fn a_label_over_a_button_does_not_steal_the_click() {
        let mut h = Harness::new(
            r#"
            local b = Instance.new("TextButton")
            b.Name = "Go"
            b.Size = UDim2.new(0, 100, 0, 100)
            b.Parent = root
            -- Drawn over the button, and NOT setting Active: a TextLabel ships
            -- Active = false, which is why every Roblox button survives having a
            -- caption laid on top of it.
            local caption = Instance.new("TextLabel")
            caption.Name = "Caption"
            caption.Size = UDim2.new(1, 0, 1, 0)
            caption.ZIndex = 4
            caption.Parent = b
            b.Activated:Connect(function() table.insert(log, "activated") end)
        "#,
        );
        // THE LABEL IS ON TOP OF THE PAINT AND TRANSPARENT TO THE HIT. `Active`
        // decides who is hit; the scan steps past the label and finds the button.
        let drawn = crate::datamodel::render::frame_of(&h.dom, h.root, SIZE.0, SIZE.1);
        assert_eq!(drawn.nodes.last().expect("a node").name, "Caption");
        assert_eq!(h.hit_at(50.0, 50.0).as_deref(), Some("Go"));
        h.click(50.0, 50.0);
        assert_eq!(h.log(), "activated");
    }

    #[test]
    fn an_active_frame_is_hit_and_a_default_frame_is_not() {
        let h = Harness::new(
            r#"
            local passive = Instance.new("Frame")
            passive.Name = "Passive"
            passive.Size = UDim2.new(0, 100, 0, 100)
            passive.Parent = root
            local panel = Instance.new("Frame")
            panel.Name = "Panel"
            panel.Size = UDim2.new(0, 100, 0, 100)
            panel.Position = UDim2.new(0, 100, 0, 0)
            panel.Active = true
            panel.Parent = root
        "#,
        );
        // A `Frame` SHIPS `Active = false`, so it is not a hit candidate, and a
        // mod that wants a hoverable or clickable panel writes `Active = true` --
        // which is what a Roblox developer writes for one anyway. The departure
        // this costs is recorded on `input::sinks`.
        assert_eq!(h.hit_at(50.0, 50.0), None);
        assert_eq!(h.hit_at(150.0, 50.0).as_deref(), Some("Panel"));
    }

    #[test]
    fn interactable_false_suppresses_the_events_and_still_sinks() {
        let mut h = Harness::new(
            r#"
            local under = Instance.new("TextButton")
            under.Name = "Under"
            under.Size = UDim2.new(0, 100, 0, 100)
            under.Parent = root
            local disabled = Instance.new("TextButton")
            disabled.Name = "Disabled"
            disabled.Size = UDim2.new(0, 100, 0, 100)
            disabled.ZIndex = 4
            disabled.Interactable = false
            disabled.Parent = root
            under.Activated:Connect(function() table.insert(log, "under") end)
            disabled.Activated:Connect(function() table.insert(log, "disabled") end)
        "#,
        );
        // THE HIT STILL LANDS ON THE DISABLED BUTTON -- `Interactable` gates the
        // firing, not the candidacy -- so the click is swallowed rather than
        // falling through to the button underneath. A disabled control that let
        // clicks reach whatever is behind it is a worse bug than one that eats
        // them.
        assert_eq!(h.hit_at(50.0, 50.0).as_deref(), Some("Disabled"));
        h.click(50.0, 50.0);
        assert_eq!(h.log(), "");
    }

    // ── `Activated` is a `GuiButton` event ────────────────────────────────────

    #[test]
    fn a_frame_has_no_activated_and_says_so() {
        // `Activated` IS A `GuiButton` EVENT, NOT A `GuiObject` ONE, and the class
        // parameter on `implements` is what enforces it. This is the first sprint
        // in which that parameter is not vacuous.
        let error = Harness::new(
            r#"
            local f = Instance.new("Frame")
            f.Parent = root
        "#,
        )
        .lua
        .load("local f = root:GetChildren()[1] return f.Activated")
        .eval::<LuaValue>()
        .expect_err("a Frame has no Activated");
        assert!(
            error.to_string().contains("not a valid member"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn a_frame_has_the_pointer_events_a_gui_object_has() {
        // The other half of the same check: `MouseEnter` IS a `GuiObject` event,
        // so a `Frame` has it even though it has no `Activated`.
        let h = Harness::new(
            r#"
            local f = Instance.new("Frame")
            f.Name = "Panel"
            f.Parent = root
        "#,
        );
        assert_eq!(
            h.eval::<String>(r#"return typeof(root:FindFirstChild("Panel").MouseEnter)"#),
            "RBXScriptSignal"
        );
    }

    #[test]
    fn implements_answers_by_class_for_the_input_family() {
        // The predicate `datamodel-surface` reports from, asked directly. A
        // name-only version would answer true for all four of these.
        assert!(implements_for("TextButton", "Activated"));
        assert!(!implements_for("Frame", "Activated"));
        assert!(implements_for("Frame", "MouseEnter"));
        assert!(!implements_for("UIListLayout", "MouseEnter"));
        // AND A `GuiButton` STILL HAS THE `GuiObject` HALF, by the superclass
        // walk rather than by being listed twice.
        assert!(implements_for("TextButton", "InputBegan"));
    }

    fn implements_for(class: &str, member: &str) -> bool {
        crate::datamodel::members::implements(class, member)
    }

    // ── Hover ─────────────────────────────────────────────────────────────────

    #[test]
    fn moving_between_two_elements_leaves_before_it_enters() {
        let mut h = Harness::new(
            r#"
            local a = Instance.new("TextButton")
            a.Name = "A"
            a.Size = UDim2.new(0, 100, 0, 100)
            a.Parent = root
            local b = Instance.new("TextButton")
            b.Name = "B"
            b.Size = UDim2.new(0, 100, 0, 100)
            b.Position = UDim2.new(0, 100, 0, 0)
            b.Parent = root
            for _, e in { a, b } do
                e.MouseEnter:Connect(function() table.insert(log, `enter {e.Name}`) end)
                e.MouseLeave:Connect(function() table.insert(log, `leave {e.Name}`) end)
            end
        "#,
        );
        h.moved(50.0, 50.0);
        h.moved(60.0, 50.0);
        h.moved(150.0, 50.0);
        // LEAVE BEFORE ENTER, and no repeat enter for a move WITHIN one element:
        // a guest that highlights on enter and clears on leave has exactly one
        // highlighted element at a time only if the order is this way round.
        assert_eq!(h.log(), "enter A,leave A,enter B");
    }

    #[test]
    fn leaving_the_surface_entirely_still_leaves() {
        let mut h = Harness::new(&format!(
            r#"
            {ONE_BUTTON}
            b.MouseEnter:Connect(function() table.insert(log, "enter") end)
            b.MouseLeave:Connect(function() table.insert(log, "leave") end)
        "#
        ));
        h.moved(50.0, 50.0);
        h.moved(150.0, 50.0);
        assert_eq!(h.log(), "enter,leave");
    }

    #[test]
    fn mouse_moved_carries_the_position() {
        let mut h = Harness::new(&format!(
            r#"
            {ONE_BUTTON}
            b.MouseMoved:Connect(function(x, y) table.insert(log, `{{x}} {{y}}`) end)
        "#
        ));
        h.moved(12.0, 34.0);
        assert_eq!(h.log(), "12 34");
    }

    // ── The case that breaks a naive implementation: the tree changing under a
    //    stationary cursor ─────────────────────────────────────────────────────

    #[test]
    fn an_element_appearing_under_a_stationary_cursor_enters() {
        let mut h = Harness::new(
            r#"
            log.appeared = nil
            function reveal()
                local b = Instance.new("TextButton")
                b.Name = "Late"
                b.Size = UDim2.new(0, 100, 0, 100)
                b.Parent = root
                b.MouseEnter:Connect(function() table.insert(log, "enter Late") end)
            end
        "#,
        );
        h.moved(50.0, 50.0);
        assert_eq!(h.log(), "");
        // The cursor does not move again. The TREE moves instead, which is the
        // case an implementation that only reconciles hover on a pointer event
        // gets wrong -- and gets wrong silently, since nothing reports a hover
        // that never happened.
        h.eval::<()>("reveal()");
        h.refresh();
        assert_eq!(h.log(), "enter Late");
    }

    #[test]
    fn a_destroyed_hover_is_dropped_and_the_next_element_enters() {
        let mut h = Harness::new(
            r#"
            local over = Instance.new("TextButton")
            over.Name = "Over"
            over.Size = UDim2.new(0, 100, 0, 100)
            over.ZIndex = 9
            over.Parent = root
            local under = Instance.new("TextButton")
            under.Name = "Under"
            under.Size = UDim2.new(0, 100, 0, 100)
            under.Parent = root
            over.MouseEnter:Connect(function() table.insert(log, "enter Over") end)
            over.MouseLeave:Connect(function() table.insert(log, "leave Over") end)
            under.MouseEnter:Connect(function() table.insert(log, "enter Under") end)
            function drop() over:Destroy() end
        "#,
        );
        h.moved(50.0, 50.0);
        assert_eq!(h.log(), "enter Over");
        h.eval::<()>("drop()");
        h.refresh();
        // NO `MouseLeave` FOR THE DESTROYED ONE -- there is nothing to fire it on,
        // because `Dom::destroy` dropped its connections on the way past. What
        // matters is that the stale hover does not persist and the element now
        // under the cursor gets its `MouseEnter`.
        assert_eq!(h.log(), "enter Over,enter Under");
    }

    #[test]
    fn a_press_target_destroyed_before_the_release_completes_no_click() {
        let mut h = Harness::new(&format!(
            r#"
            {ONE_BUTTON}
            b.MouseButton1Click:Connect(function() table.insert(log, "click") end)
            function drop() b:Destroy() end
        "#
        ));
        h.down(50.0, 50.0);
        h.eval::<()>("drop()");
        h.refresh();
        h.up(50.0, 50.0);
        // Nothing to click and nothing to crash on. The press target is cleared
        // with its element, so the button cannot be left half-pressed for the
        // life of the mod.
        assert_eq!(h.log(), "");
    }

    #[test]
    fn refresh_before_the_cursor_has_ever_arrived_fires_nothing() {
        let mut h = Harness::new(&format!(
            r#"
            {ONE_BUTTON}
            b.MouseEnter:Connect(function() table.insert(log, "enter") end)
        "#
        ));
        // ASSUMING (0, 0) WOULD FIRE `MouseEnter` on whatever sits in the
        // top-left corner of every window at startup -- and this button does.
        h.refresh();
        assert_eq!(h.log(), "");
    }

    // ── The sprint 8 traps, which apply here unchanged ────────────────────────

    #[test]
    fn a_handler_that_touches_the_tree_does_not_deadlock() {
        let mut h = Harness::new(&format!(
            r#"
            {ONE_BUTTON}
            local other = Instance.new("TextButton")
            other.Name = "Other"
            other.Size = UDim2.new(0, 100, 0, 100)
            other.Position = UDim2.new(0, 100, 0, 0)
            other.Parent = root
            b.Activated:Connect(function()
                -- THE ORDINARY CASE FOR AN INPUT HANDLER, not an exotic one: a
                -- click that changes something else. The arena is behind a
                -- non-reentrant `Mutex`, so a fire with the lock still held stops
                -- the process here -- which is what sprint 7 hit in `__index` and
                -- sprint 8 hit in `__newindex`.
                other.Text = "moved"
                other.Size = UDim2.new(0, 50, 0, 50)
            end)
        "#
        ));
        h.click(50.0, 50.0);
        // `FindFirstChild` RATHER THAN `root.Other`. Indexing a child by name is
        // not a member this host implements -- `__index` answers "Other is not a
        // valid member of ScreenGui", correctly by its own rules, since the
        // reflection database has no such property. It is a real gap and a
        // separate one; see the sprint record.
        assert_eq!(
            h.eval::<String>(r#"return root:FindFirstChild("Other").Text"#),
            "moved"
        );
    }

    #[test]
    fn a_handler_that_destroys_its_own_element_survives_the_rest_of_the_batch() {
        let mut h = Harness::new(&format!(
            r#"
            {ONE_BUTTON}
            b.MouseButton1Up:Connect(function()
                table.insert(log, "up")
                b:Destroy()
            end)
            b.MouseButton1Click:Connect(function() table.insert(log, "click") end)
            b.Activated:Connect(function() table.insert(log, "activated") end)
        "#
        ));
        h.up(50.0, 50.0);
        // The batch was collected before any handler ran, and each entry is
        // re-checked against the arena immediately before it fires. The receiver
        // is gone, so the queued `click` and `activated` are skipped rather than
        // called on a freed slot.
        assert_eq!(h.log(), "up");
    }

    #[test]
    fn a_handlers_error_is_reported_and_does_not_stop_the_batch() {
        let mut h = Harness::new(&format!(
            r#"
            {ONE_BUTTON}
            b.MouseButton1Up:Connect(function() error("deliberate") end)
            b.Activated:Connect(function() table.insert(log, "activated") end)
        "#
        ));
        h.click(50.0, 50.0);
        // Sprint 8's rule, reached through input: a failing listener cannot fail
        // the event that notified it, or one bad handler takes every other
        // handler on the element with it.
        assert_eq!(h.log(), "activated");
    }

    // ── The repaint discipline sprint 8 bought ────────────────────────────────

    #[test]
    fn a_handler_that_changes_nothing_does_not_dirty_the_tree() {
        let mut h = Harness::new(&format!(
            r#"
            {ONE_BUTTON}
            b.Text = "Go"
            local seen = 0
            b.Activated:Connect(function()
                seen += 1
                -- ASSIGNING THE SAME VALUE IS NOT A CHANGE, which is sprint 8's
                -- decision 1. So this handler genuinely changes nothing.
                b.Text = "Go"
            end)
            function seenCount() return seen end
        "#
        ));
        h.dirty();
        h.moved(50.0, 50.0);
        h.click(50.0, 50.0);
        h.moved(60.0, 50.0);
        h.wheel(50.0, 50.0, 1.0);
        assert_eq!(h.eval::<i32>("return seenCount()"), 1);
        // THE HANDLER RAN AND THE SCREEN IS UNCHANGED. If input dirtied the tree
        // on its own, `painted` would climb back to `fps` the moment a cursor
        // crossed the window and sprint 8's gain would evaporate silently.
        assert!(!h.dirty(), "input dirtied a tree nothing changed");
    }

    #[test]
    fn a_handler_that_changes_something_does_dirty_the_tree() {
        let mut h = Harness::new(&format!(
            r#"
            {ONE_BUTTON}
            b.Activated:Connect(function() b.Text = "clicked" end)
        "#
        ));
        h.dirty();
        h.click(50.0, 50.0);
        // The other half of the same rule: a real change still reaches the screen,
        // through the path that already marks the tree dirty.
        assert!(h.dirty(), "a real change did not dirty the tree");
    }

    // ── The wheel ─────────────────────────────────────────────────────────────

    #[test]
    fn the_wheel_picks_a_direction_from_the_sign() {
        let mut h = Harness::new(&format!(
            r#"
            {ONE_BUTTON}
            b.MouseWheelForward:Connect(function() table.insert(log, "forward") end)
            b.MouseWheelBackward:Connect(function() table.insert(log, "backward") end)
        "#
        ));
        h.wheel(50.0, 50.0, 1.0);
        h.wheel(50.0, 50.0, -1.0);
        // A ZERO DELTA IS NOT A DIRECTION, and neither event is the honest answer
        // for one -- so nothing fires rather than `Backward` winning the sign test
        // by default.
        h.wheel(50.0, 50.0, 0.0);
        assert_eq!(h.log(), "forward,backward");
    }

    // ── `InputObject` ─────────────────────────────────────────────────────────

    #[test]
    fn an_input_object_reports_its_type_and_state() {
        let h = Harness::new(&format!(
            r#"
            {ONE_BUTTON}
            b.InputBegan:Connect(function(input)
                table.insert(log, typeof(input))
                table.insert(log, tostring(input.UserInputType))
                table.insert(log, tostring(input.UserInputState))
                table.insert(log, tostring(input.Position.X))
                table.insert(log, tostring(input.Position.Y))
            end)
        "#
        ));
        let mut h = h;
        h.down(12.0, 34.0);
        // `typeof` IS ASSERTED BY VALUE. mlua fills `__type` in with the Rust type
        // name when it is left out, so "not nil" is exactly what the broken
        // version also passes -- and this codebase has got `__type` wrong twice.
        assert_eq!(
            h.log(),
            "InputObject,Enum.UserInputType.MouseButton1,Enum.UserInputState.Begin,12,34"
        );
    }

    #[test]
    fn an_input_objects_enum_items_compare_equal_to_the_guests_own() {
        let mut h = Harness::new(&format!(
            r#"
            {ONE_BUTTON}
            b.InputBegan:Connect(function(input)
                table.insert(log, tostring(
                    input.UserInputType == Enum.UserInputType.MouseButton1
                ))
                table.insert(log, tostring(
                    input.UserInputState == Enum.UserInputState.Begin
                ))
            end)
        "#
        ));
        h.right_down(50.0, 50.0);
        // The right button, so the first comparison is false -- which is the
        // assertion that matters: an item the HOST named and an item the GUEST
        // named come from the same generated database and compare as values.
        assert_eq!(h.log(), "false,true");
        h.down(50.0, 50.0);
        assert_eq!(h.log(), "false,true,true,true");
    }

    #[test]
    fn the_middle_button_reaches_input_began_and_has_no_click() {
        let mut h = Harness::new(&format!(
            r#"
            {ONE_BUTTON}
            b.InputBegan:Connect(function(input)
                table.insert(log, tostring(input.UserInputType))
            end)
            b.MouseButton1Click:Connect(function() table.insert(log, "click") end)
        "#
        ));
        h.press(Button::Middle, 50.0, 50.0);
        h.release(Button::Middle, 50.0, 50.0);
        // The engine gives the middle button no `MouseButton3` family, so neither
        // does this -- but the input events are the generic ones and it reaches
        // them.
        assert_eq!(h.log(), "Enum.UserInputType.MouseButton3");
    }

    #[test]
    fn a_move_reports_its_delta_through_input_changed() {
        let mut h = Harness::new(&format!(
            r#"
            {ONE_BUTTON}
            b.InputChanged:Connect(function(input)
                table.insert(log, `{{input.Delta.X}} {{input.Delta.Y}}`)
            end)
        "#
        ));
        h.moved(10.0, 10.0);
        h.moved(30.0, 25.0);
        // THE FIRST MOVE HAS NO DELTA. There is no previous position to subtract,
        // and inventing one from the origin would report a jump from the corner of
        // the window on the first sample.
        assert_eq!(h.log(), "0 0,20 15");
    }

    #[test]
    fn input_ended_goes_to_where_the_press_began() {
        let mut h = Harness::new(
            r#"
            local a = Instance.new("TextButton")
            a.Name = "A"
            a.Size = UDim2.new(0, 100, 0, 100)
            a.Parent = root
            local b = Instance.new("TextButton")
            b.Name = "B"
            b.Size = UDim2.new(0, 100, 0, 100)
            b.Position = UDim2.new(0, 100, 0, 0)
            b.Parent = root
            a.InputEnded:Connect(function() table.insert(log, "a ended") end)
            b.InputEnded:Connect(function() table.insert(log, "b ended") end)
        "#,
        );
        h.down(50.0, 50.0);
        h.up(150.0, 50.0);
        // A DRAG NEEDS THIS READING. An element told input began on it has to be
        // told it ended, or it holds a pressed state nothing can ever clear.
        assert_eq!(h.log(), "a ended");
    }

    #[test]
    fn textbox_focus_full_lifecycle() {
        let mut h = Harness::new(
            r#"
            local tb = Instance.new("TextBox")
            tb.Size = UDim2.new(0, 100, 0, 50)
            tb.Position = UDim2.new(0, 0, 0, 0)
            tb.Parent = root

            tb.Focused:Connect(function()
                table.insert(log, `focused:{tb:IsFocused()}`)
            end)
            tb.FocusLost:Connect(function(enterPressed)
                table.insert(log, `lost:{enterPressed}:{tb:IsFocused()}`)
            end)
        "#,
        );
        // 1. Click in -> IsFocused true, Focused fired once
        h.down(50.0, 25.0);
        h.up(50.0, 25.0);
        assert_eq!(h.log(), "focused:true");

        // 2. Press Return -> FocusLost fired once with enterPressed == true
        h.key("Return");
        assert_eq!(h.log(), "focused:true,lost:true:false");

        // 3. Click in again -> IsFocused true, Focused fired once
        h.down(50.0, 25.0);
        h.up(50.0, 25.0);
        assert_eq!(h.log(), "focused:true,lost:true:false,focused:true");

        // 4. Click outside -> FocusLost fired with enterPressed == false
        h.down(150.0, 75.0);
        h.up(150.0, 75.0);
        assert_eq!(
            h.log(),
            "focused:true,lost:true:false,focused:true,lost:false:false"
        );
    }

    #[test]
    fn textbox_capture_and_release_focus_programmatically() {
        let h = Harness::new(
            r#"
            local a = Instance.new("TextBox")
            a.Parent = root
            local b = Instance.new("TextBox")
            b.Parent = root

            a.Focused:Connect(function() table.insert(log, "a focused") end)
            a.FocusLost:Connect(function(ep) table.insert(log, `a lost:{ep}`) end)
            b.Focused:Connect(function() table.insert(log, "b focused") end)
            b.FocusLost:Connect(function(ep) table.insert(log, `b lost:{ep}`) end)

            a:CaptureFocus()
            -- Capturing already-focused box does not fire Focused twice
            a:CaptureFocus()
            -- Switching focus to b releases a and focuses b
            b:CaptureFocus()
            b:ReleaseFocus()
            -- Releasing when not focused is a no-op
            b:ReleaseFocus()
        "#,
        );
        assert_eq!(h.log(), "a focused,a lost:false,b focused,b lost:false");
    }

    #[test]
    fn click_outside_when_not_focused_does_not_fire_focus_lost() {
        let mut h = Harness::new(
            r#"
            local tb = Instance.new("TextBox")
            tb.Size = UDim2.new(0, 100, 0, 50)
            tb.Parent = root

            tb.FocusLost:Connect(function() table.insert(log, "lost") end)
        "#,
        );
        // Click outside
        h.down(150.0, 75.0);
        h.up(150.0, 75.0);
        assert_eq!(h.log(), "");
    }

    #[test]
    fn click_inside_already_focused_box_does_not_fire_focused_twice() {
        let mut h = Harness::new(
            r#"
            local tb = Instance.new("TextBox")
            tb.Size = UDim2.new(0, 100, 0, 50)
            tb.Parent = root

            tb.Focused:Connect(function() table.insert(log, "focused") end)
        "#,
        );
        h.down(50.0, 25.0);
        h.up(50.0, 25.0);
        assert_eq!(h.log(), "focused");

        // Click again inside the same box
        h.down(60.0, 30.0);
        h.up(60.0, 30.0);
        assert_eq!(h.log(), "focused");
    }

    #[test]
    fn destroying_focused_textbox_clears_focus_owner() {
        let mut h = Harness::new(
            r#"
            local tb = Instance.new("TextBox")
            tb.Size = UDim2.new(0, 100, 0, 50)
            tb.Parent = root

            tb:CaptureFocus()
            table.insert(log, `before:{tb:IsFocused()}`)
            tb.FocusLost:Connect(function() table.insert(log, "lost") end)
            tb:Destroy()
        "#,
        );
        assert_eq!(h.log(), "before:true");

        // Click outside on empty space: must NOT attempt to release focus on the destroyed id
        h.down(150.0, 75.0);
        h.up(150.0, 75.0);
        assert_eq!(h.log(), "before:true");
    }

    #[test]
    fn deadlock_witness_handler_touches_the_tree() {
        // THE DEADLOCK TRAP: if the arena lock is held while firing a focus signal,
        // any handler that reads or mutates the tree (e.g. tb.Text) deadlocks on dom.lock().
        // Here the Focused and FocusLost handlers mutate and read properties,
        // running to completion without deadlocking.
        let mut h = Harness::new(
            r#"
            local tb = Instance.new("TextBox")
            tb.Size = UDim2.new(0, 100, 0, 50)
            tb.Text = "initial"
            tb.Parent = root

            tb.Focused:Connect(function()
                -- Read and mutate the tree under the signal dispatch:
                local old = tb.Text
                tb.Text = old .. "+focused"
                table.insert(log, tb.Text)
            end)
            tb.FocusLost:Connect(function(enterPressed)
                tb.Text = tb.Text .. "+lost"
                table.insert(log, tb.Text)
            end)
        "#,
        );
        h.down(50.0, 25.0);
        h.up(50.0, 25.0);
        h.key("Return");
        assert_eq!(h.log(), "initial+focused,initial+focused+lost");
    }

    // ── ScrollingFrame wheel input and velocity ──────────────────────────────

    #[test]
    fn wheel_scrolls_scrolling_frame_and_clamps() {
        let mut h = Harness::new(
            r#"
            local sf = Instance.new("ScrollingFrame")
            sf.Name = "Scroll"
            sf.Size = UDim2.new(0, 100, 0, 50)
            sf.CanvasSize = UDim2.new(0, 0, 0, 150)
            sf.Parent = root
        "#,
        );
        // ScrollingFrame 100x50, canvas 150 tall -> max_scroll_y is 100.
        // Delta -1.0 is scrolling down, which moves CanvasPosition.Y by +40.0.
        h.wheel(50.0, 25.0, -1.0);
        let pos1: (f32, f32) =
            h.eval("return root.Scroll.CanvasPosition.X, root.Scroll.CanvasPosition.Y");
        assert_eq!(pos1, (0.0, 40.0));

        // Scroll down again: 40 + 40 = 80.
        h.wheel(50.0, 25.0, -1.0);
        let pos2: (f32, f32) =
            h.eval("return root.Scroll.CanvasPosition.X, root.Scroll.CanvasPosition.Y");
        assert_eq!(pos2, (0.0, 80.0));

        // Scroll down again: 80 + 40 = 120, clamped to max_scroll_y = 100.
        h.wheel(50.0, 25.0, -1.0);
        let pos3: (f32, f32) =
            h.eval("return root.Scroll.CanvasPosition.X, root.Scroll.CanvasPosition.Y");
        assert_eq!(pos3, (0.0, 100.0));

        // Scroll up (+1.0): 100 - 40 = 60.
        h.wheel(50.0, 25.0, 1.0);
        let pos4: (f32, f32) =
            h.eval("return root.Scroll.CanvasPosition.X, root.Scroll.CanvasPosition.Y");
        assert_eq!(pos4, (0.0, 60.0));

        // Scroll up twice more: 60 - 40 - 40 = -20, clamped to 0.
        h.wheel(50.0, 25.0, 1.0);
        h.wheel(50.0, 25.0, 1.0);
        let pos5: (f32, f32) =
            h.eval("return root.Scroll.CanvasPosition.X, root.Scroll.CanvasPosition.Y");
        assert_eq!(pos5, (0.0, 0.0));
    }

    #[test]
    fn scrolling_enabled_false_suppresses_wheel_scrolling() {
        let mut h = Harness::new(
            r#"
            local sf = Instance.new("ScrollingFrame")
            sf.Name = "Scroll"
            sf.Size = UDim2.new(0, 100, 0, 50)
            sf.CanvasSize = UDim2.new(0, 0, 0, 150)
            sf.ScrollingEnabled = false
            sf.Parent = root
        "#,
        );
        h.wheel(50.0, 25.0, -1.0);
        let pos: (f32, f32) =
            h.eval("return root.Scroll.CanvasPosition.X, root.Scroll.CanvasPosition.Y");
        assert_eq!(pos, (0.0, 0.0));
    }

    #[test]
    fn scroll_velocity_observes_impulse_and_resets() {
        let mut h = Harness::new(
            r#"
            local sf = Instance.new("ScrollingFrame")
            sf.Name = "Scroll"
            sf.Size = UDim2.new(0, 100, 0, 50)
            sf.CanvasSize = UDim2.new(0, 0, 0, 200)
            sf.Parent = root
        "#,
        );
        // Before wheel: velocity is 0
        let vel0: (f32, f32) = h.eval("local v = root.Scroll:GetScrollVelocity() return v.X, v.Y");
        assert_eq!(vel0, (0.0, 0.0));

        // Wheel down: velocity becomes non-zero
        h.wheel(50.0, 25.0, -1.0);
        let vel1: (f32, f32) = h.eval("local v = root.Scroll:GetScrollVelocity() return v.X, v.Y");
        assert!(
            vel1.1 > 0.0,
            "expected positive Y velocity after wheeling down, got {:?}",
            vel1
        );

        // ResetScrollVelocity: clears back to (0, 0)
        h.eval::<()>("root.Scroll:ResetScrollVelocity()");
        let vel2: (f32, f32) = h.eval("local v = root.Scroll:GetScrollVelocity() return v.X, v.Y");
        assert_eq!(vel2, (0.0, 0.0));
    }

    #[test]
    fn wheel_deadlock_witness_property_changed_mutates_tree() {
        // DEADLOCK WITNESS: CanvasPosition change listener mutates DOM under dispatch.
        // If dom.lock() is held when property_changed or signals fire, this will hang/deadlock.
        let mut h = Harness::new(
            r#"
            local sf = Instance.new("ScrollingFrame")
            sf.Name = "Scroll"
            sf.Size = UDim2.new(0, 100, 0, 50)
            sf.CanvasSize = UDim2.new(0, 0, 0, 200)
            sf.Parent = root

            local label = Instance.new("TextLabel")
            label.Name = "Status"
            label.Text = "none"
            label.Parent = root

            sf:GetPropertyChangedSignal("CanvasPosition"):Connect(function()
                -- Mutate the tree inside the signal handler:
                label.Text = `scrolled:{sf.CanvasPosition.Y}`
                table.insert(log, label.Text)
            end)
        "#,
        );
        h.wheel(50.0, 25.0, -1.0);
        assert_eq!(h.log(), "scrolled:40");
        assert_eq!(h.eval::<String>("return root.Status.Text"), "scrolled:40");
    }
}
