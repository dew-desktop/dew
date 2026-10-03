//! Does Dew's renderer honour this property on this class?
//!
//! The third predicate the host answers about its own surface, beside
//! [`crate::datamodel::accepts`] (would an assignment be stored) and
//! [`crate::datamodel::members::implements`] (would a call be dispatched). This
//! one asks whether the solver in `render.rs` and the display list it builds
//! READ the property on an instance of the class, so that changing it changes
//! what is drawn or where.
//!
//! CLASS-SCOPED, because the renderer is. `FillDirection` is read on a
//! `UIListLayout` and a `UIGridLayout` and on nothing else, `ClipsDescendants`
//! is read on a `Frame` and overridden on a `ScrollingFrame`, and `Rotation` is
//! read on a `UIGradient` and on no `GuiObject`. A flat list of names cannot say
//! any of that, and one once credited the table layout with the list's
//! properties because they share a name.
//!
//! Inherited properties are answered per concrete class. Where the renderer
//! groups classes (the ones that draw text, the ones that draw an image) this
//! asks the same functions the renderer asks, so a class that starts drawing
//! text starts being answered for it here in the same diff.
//!
//! A property the renderer never reads is [`Honour::Absent`], including one
//! that has no paint of its own (`Active`, a modifier's `Name`). Every claim of
//! [`Honour::Implemented`] or [`Honour::Partial`] is checked against evidence by
//! `every_claim_has_evidence` below.

use super::{draws_image, draws_text};
use crate::datamodel::members::class_is_a;

/// What the renderer does with one property on one class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Honour {
    /// Read, and drawn as the engine draws it as far as the conformance cases
    /// and the code's own stated rules reach.
    Implemented,
    /// Read, with part of its meaning dropped. The text says which part.
    Partial(&'static str),
    /// Never read. Setting it changes nothing that is drawn.
    Absent,
}

impl Honour {
    /// Implemented or partial: the renderer reads it at all.
    pub fn rendered(self) -> bool {
        !matches!(self, Honour::Absent)
    }
}

/// Does Dew's renderer honour `property` on an instance of `class`?
pub fn honours(class: &str, property: &str) -> Honour {
    if class_is_a(class, "GuiObject") {
        return gui_object(class, property);
    }
    match class {
        "UIListLayout" => list_layout(property),
        "UIGridLayout" => grid_layout(property),
        "UIFlexItem" => flex_item(property),
        "UISizeConstraint" => size_constraint(property),
        "UIAspectRatioConstraint" => aspect_ratio_constraint(property),
        "UIPadding" => padding(property),
        "UICorner" => corner(property),
        "UIStroke" => stroke(property),
        "UIGradient" => gradient(property),
        // `UITableLayout`, `UIPageLayout`, `UIScale` and `UITextSizeConstraint`
        // are skipped as modifiers: the solver never looks for them, so nothing
        // on them is read, `Parent` included.
        _ => Honour::Absent,
    }
}

/// Everything under `GuiObject`.
fn gui_object(class: &str, property: &str) -> Honour {
    use Honour::*;
    match property {
        // `Parent` is the tree `visit` walks. `Name` is the sort key when the
        // parent's list or grid sorts by `Name` (`layout_order`).
        "Parent" | "Name" => return Implemented,
        // `solve_rect`, `visit` and `grow`.
        "Position" | "Size" | "AnchorPoint" | "AutomaticSize" | "Visible" => return Implemented,
        // `paint_rank` orders siblings by it.
        "ZIndex" => return Implemented,
        // The sort key when the parent's list or grid sorts by `LayoutOrder`.
        "LayoutOrder" => return Implemented,
        // `node`: the fill and its alpha.
        "BackgroundColor3" | "BackgroundTransparency" => return Implemented,
        // `visit` clips a `ScrollingFrame` whatever this says, so on that
        // class it is not read; what the engine does when it is false there is
        // unverified.
        "ClipsDescendants" => {
            return if class_is_a(class, "ScrollingFrame") {
                Absent
            } else {
                Implemented
            }
        }
        _ => {}
    }

    if class_is_a(class, "ScrollingFrame")
        && matches!(
            property,
            "CanvasSize" | "CanvasPosition" | "AutomaticCanvasSize"
        )
    {
        return Implemented;
    }

    if draws_text(class) {
        match property {
            "Text" | "TextColor3" | "TextSize" | "TextTransparency" | "TextXAlignment"
            | "TextYAlignment" | "TextScaled" | "TextWrapped" => return Implemented,
            // `face_of` resolves the face through `crate::fonts`. Assigning
            // `Font` sets `FontFace`, so both reach the same face.
            "FontFace" | "Font" => {
                return Partial(
                    "a family Dew ships, or finds in a local Studio install, is drawn in \
                     its own face; any other is drawn in the nearest shipped face",
                )
            }
            "InputAction" => {
                return Partial("drawn as its own name in text; no glyph for the bound input")
            }
            _ => {}
        }
    }

    // `text_layout_of` reads truncation, line spacing and markup, and `node`
    // the stroke, on the three classes that report `TextBounds`.
    if matches!(class, "TextLabel" | "TextButton" | "TextBox") {
        match property {
            "LineHeight" | "TextStrokeColor3" | "TextStrokeTransparency" => return Implemented,
            "TextTruncate" => {
                return Partial("AtEnd is drawn with an ellipsis; SplitWord is drawn as None")
            }
            "RichText" => {
                return Partial(
                    "markup is stripped and measured; bold, italic, colour and other tags \
                     are not drawn",
                )
            }
            _ => {}
        }
    }

    // `InputActionLabel` draws an image only if a guest gives it an `Image`,
    // which the engine's class does not have, so its tint and transparency
    // reach nothing the engine would draw.
    if draws_image(class) && class != "InputActionLabel" {
        match property {
            "Image" | "ImageContent" | "ImageColor3" | "ImageTransparency" | "ImageRectOffset"
            | "ImageRectSize" => return Implemented,
            "ScaleType" => {
                return Partial(
                    "Stretch, Fit and Crop are drawn; Slice and Tile fall back to Stretch",
                )
            }
            _ => {}
        }
    }

    Absent
}

fn list_layout(property: &str) -> Honour {
    match property {
        // `ListLayout::read` and `place_list`. `Parent` is how the solver finds
        // it: the first `UIListLayout` among a node's children.
        "Parent"
        | "FillDirection"
        | "SortOrder"
        | "Padding"
        | "HorizontalAlignment"
        | "VerticalAlignment"
        | "HorizontalFlex"
        | "VerticalFlex"
        | "Wraps"
        | "ItemLineAlignment" => Honour::Implemented,
        _ => Honour::Absent,
    }
}

fn grid_layout(property: &str) -> Honour {
    match property {
        // `place_grid`.
        "Parent"
        | "CellSize"
        | "CellPadding"
        | "FillDirection"
        | "FillDirectionMaxCells"
        | "StartCorner"
        | "SortOrder"
        | "HorizontalAlignment"
        | "VerticalAlignment" => Honour::Implemented,
        _ => Honour::Absent,
    }
}

fn flex_item(property: &str) -> Honour {
    match property {
        // `flex_item_of`, read for a child of a `UIListLayout`'s parent.
        "Parent" | "FlexMode" | "GrowRatio" | "ShrinkRatio" | "ItemLineAlignment" => {
            Honour::Implemented
        }
        _ => Honour::Absent,
    }
}

fn size_constraint(property: &str) -> Honour {
    match property {
        // `size_bounds`.
        "Parent" | "MinSize" | "MaxSize" => Honour::Implemented,
        _ => Honour::Absent,
    }
}

fn aspect_ratio_constraint(property: &str) -> Honour {
    match property {
        // `aspect_size`.
        "Parent" | "AspectRatio" | "AspectType" => Honour::Implemented,
        "DominantAxis" => Honour::Partial(
            "read only under ScaleWithParentSize; FitWithinMaxSize ignores it, which is unverified",
        ),
        _ => Honour::Absent,
    }
}

fn padding(property: &str) -> Honour {
    match property {
        "Parent" => Honour::Implemented,
        // `padding_of` keeps the offset of each `UDim`.
        "PaddingLeft" | "PaddingTop" | "PaddingRight" | "PaddingBottom" => {
            Honour::Partial("the offset insets the content; the scale is discarded")
        }
        _ => Honour::Absent,
    }
}

fn corner(property: &str) -> Honour {
    match property {
        "Parent" => Honour::Implemented,
        // `corner_radius` keeps the offset; the four per-corner radii are
        // never read.
        "CornerRadius" => Honour::Partial("the offset rounds the corners; the scale is dropped"),
        _ => Honour::Absent,
    }
}

fn stroke(property: &str) -> Honour {
    match property {
        // `strokes_of`, which picks the box or the glyphs. `Enabled` is not
        // read, so a disabled stroke still draws.
        "Parent" | "Color" | "ApplyStrokeMode" => Honour::Implemented,
        "Thickness" => Honour::Partial(
            "around glyphs, drawn as copies of the text stamped out to the thickness, \
             so joins are always round",
        ),
        "Transparency" => Honour::Partial(
            "around glyphs, the stamped copies overlap, so a translucent outline reads \
             more solid than the engine's",
        ),
        _ => Honour::Absent,
    }
}

fn gradient(property: &str) -> Honour {
    match property {
        // `gradient_of` and `gradient_kind`.
        "Parent" | "Color" | "Transparency" | "Rotation" | "Enabled" | "Type" => {
            Honour::Implemented
        }
        _ => Honour::Absent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{conformance, gallery, scope};
    use mlua::{Lua, Table};
    use std::collections::{BTreeMap, BTreeSet};

    type Pair = (String, String);

    /// Can a guest make an instance of this class? A class the reflection
    /// database does not carry is one this host synthesizes, which it can.
    fn creatable(class: &str) -> bool {
        let Ok(db) = rbx_reflection_database::get() else {
            return true;
        };
        db.classes
            .get(class)
            .map(|c| !c.tags.contains(&rbx_reflection::ClassTag::NotCreatable))
            .unwrap_or(true)
    }

    /// Every pair with evidence that the renderer reads it, and where from.
    ///
    /// TWO SOURCES, BOTH RUN HERE RATHER THAN TRUSTED. A gallery variant on
    /// that class that changed the image, and a conformance case with
    /// `roblox` provenance that passes on this host and whose tree sets the
    /// property on that class.
    fn evidence() -> BTreeMap<Pair, String> {
        let mut found: BTreeMap<Pair, String> = BTreeMap::new();

        let scenes_dir = gallery::find_scenes_dir(None).expect("gallery scenes");
        let lua = Lua::new();
        let scenes = gallery::load_all(&lua, &scenes_dir).expect("scenes load");
        let diff = gallery::differential(&scenes).expect("differential pass");
        for (pair, source) in diff.moved {
            found.entry(pair).or_insert(format!("gallery {source}"));
        }

        let cases_dir = conformance::find_cases_dir(None).expect("conformance cases");
        let (results, _) = conformance::run_suite(&cases_dir, None);
        for result in results {
            if result.provenance != "roblox" || result.status != conformance::CaseStatus::Pass {
                continue;
            }
            let lua = Lua::new();
            let path = cases_dir.join(format!("{}.luau", result.file_stem));
            let case = conformance::decode_case(&lua, &path).expect("a passing case decodes");
            let tree: Table = lua.registry_value(&case.tree_val).expect("tree");
            for pair in gallery::pairs_in_tree(&tree).expect("tree walks") {
                found
                    .entry(pair)
                    .or_insert(format!("conformance {}", result.file_stem));
            }
        }
        found
    }

    /// The claims with nothing behind them, one line each.
    ///
    /// A claim stands on evidence for its own pair, or on an entry in the
    /// gallery's `CANNOT_DIFFER`, which states why pixels are the wrong
    /// question. A class no guest can instantiate (`GuiObject`, `GuiButton`,
    /// `GuiLabel`) has no instance to vary, so its claim stands on the same
    /// answer for the same property on a concrete class under it, and that
    /// claim has to stand on its own.
    fn unevidenced(
        by_class: &BTreeMap<String, BTreeSet<String>>,
        answer: impl Fn(&str, &str) -> Honour,
        evidence: &BTreeMap<Pair, String>,
    ) -> Vec<String> {
        let direct = |class: &str, property: &str| {
            evidence.contains_key(&(class.to_string(), property.to_string()))
                || gallery::excused(property).is_some()
        };
        let mut missing = Vec::new();
        for (class, props) in by_class {
            for property in props {
                let claim = answer(class, property);
                if !claim.rendered() || direct(class, property) {
                    continue;
                }
                let borrowed = !creatable(class)
                    && by_class.iter().any(|(concrete, theirs)| {
                        creatable(concrete)
                            && class_is_a(concrete, class)
                            && theirs.contains(property)
                            && answer(concrete, property) == claim
                            && direct(concrete, property)
                    });
                if !borrowed {
                    missing.push(format!("{class}.{property} claimed {claim:?}"));
                }
            }
        }
        missing
    }

    /// EVERY CLAIM HAS EVIDENCE. A predicate answering `Implemented` for
    /// everything would produce numbers too; this is what stops it.
    ///
    /// Skipped loudly without the interface surface, which CI generates.
    #[test]
    fn every_claim_has_evidence() {
        let Some(by_class) = scope::in_scope_by_class() else {
            eprintln!("SKIPPED: {}", scope::API_SURFACE_MISSING);
            return;
        };
        let evidence = evidence();
        let missing = unevidenced(&by_class, honours, &evidence);
        assert!(
            missing.is_empty(),
            "{} claim(s) with no gallery variant that moved pixels, no passing roblox \
             conformance case setting the property on the class, and no excuse:\n  {}",
            missing.len(),
            missing.join("\n  ")
        );
    }

    /// THE CHECK CAN FAIL. `UITableLayout` is never laid out, so claiming its
    /// `FillDirection` must be caught, and evidence for the list's
    /// `FillDirection` must not cover it.
    #[test]
    fn a_claim_on_a_class_the_solver_skips_is_caught() {
        let mut by_class: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for class in ["UIListLayout", "UITableLayout"] {
            by_class.insert(class.to_string(), ["FillDirection".to_string()].into());
        }
        let evidence: BTreeMap<Pair, String> = [(
            ("UIListLayout".to_string(), "FillDirection".to_string()),
            "a list variant".to_string(),
        )]
        .into();
        let lying = |class: &str, property: &str| {
            if class == "UITableLayout" {
                Honour::Implemented
            } else {
                honours(class, property)
            }
        };
        assert_eq!(
            unevidenced(&by_class, lying, &evidence),
            vec!["UITableLayout.FillDirection claimed Implemented".to_string()]
        );
        assert!(unevidenced(&by_class, honours, &evidence).is_empty());
    }

    /// One name, different answers, because the renderer reads it on one class
    /// and not another.
    #[test]
    fn the_answer_is_scoped_to_the_class() {
        assert_eq!(
            honours("UIListLayout", "FillDirection"),
            Honour::Implemented
        );
        assert_eq!(
            honours("UIGridLayout", "FillDirection"),
            Honour::Implemented
        );
        assert_eq!(honours("UITableLayout", "FillDirection"), Honour::Absent);
        assert_eq!(honours("UIListLayout", "Padding"), Honour::Implemented);
        assert_eq!(honours("UITableLayout", "Padding"), Honour::Absent);
        assert_eq!(honours("UIPageLayout", "Padding"), Honour::Absent);
        assert_eq!(honours("UIGradient", "Rotation"), Honour::Implemented);
        assert_eq!(honours("Frame", "Rotation"), Honour::Absent);
        assert_eq!(honours("Frame", "ClipsDescendants"), Honour::Implemented);
        assert_eq!(
            honours("ScrollingFrame", "ClipsDescendants"),
            Honour::Absent
        );
        assert_eq!(honours("ImageLabel", "ImageColor3"), Honour::Implemented);
        assert_eq!(honours("InputActionLabel", "ImageColor3"), Honour::Absent);
        assert_eq!(honours("UIFlexItem", "Parent"), Honour::Implemented);
        assert_eq!(honours("UITableLayout", "Parent"), Honour::Absent);
    }

    /// A partial answer says what is missing, in words a reader of the scope
    /// document can act on.
    #[test]
    fn every_partial_says_what_is_missing() {
        let Some(by_class) = scope::in_scope_by_class() else {
            eprintln!("SKIPPED: {}", scope::API_SURFACE_MISSING);
            return;
        };
        for (class, props) in &by_class {
            for property in props {
                if let Honour::Partial(reason) = honours(class, property) {
                    assert!(
                        reason.len() > 30,
                        "{class}.{property} is partial with no real reason: {reason:?}"
                    );
                }
            }
        }
    }
}
