//! An overlay must sit above sticky chrome, in every product.
//!
//! A menu opening from a footer once slid under the navbar. The cause was not
//! a low number: the `z-index` was on the popup, which is not positioned, and
//! the positioner that *is* placed had none, so it stayed at `auto` and lost to
//! a `z-50` header however high the popup's own value was. These read the
//! shipped templates and hold the one rule that prevents it: the layer goes on
//! the positioner, and it is the shared `--z-overlay`.

const MARKS: &str = include_str!("../capabilities/design/apps/web/src/app/marks.css");
const DROPDOWN: &str = include_str!(
    "../capabilities/design/for/web-next/apps/web/src/components/ui/dropdown-menu.tsx"
);
const PICKER_TSX: &str =
    include_str!("../capabilities/i18n/for/web-next/apps/web/src/components/locale-picker.tsx");
const PICKER_CSS: &str = include_str!(
    "../capabilities/i18n/for/web-next/apps/web/src/components/locale-picker.module.css"
);

/// The text of the first rule whose selector is exactly `selector`.
fn rule<'a>(css: &'a str, selector: &str) -> &'a str {
    let start = css
        .find(&format!("\n{selector} {{"))
        .unwrap_or_else(|| panic!("no `{selector}` rule"));
    let body = &css[start..];
    &body[..body.find('}').expect("unclosed rule")]
}

#[test]
fn the_overlay_layer_is_declared_once_and_above_sticky_chrome() {
    let line = MARKS
        .lines()
        .find(|l| l.trim_start().starts_with("--z-overlay:"))
        .expect("marks.css declares --z-overlay");
    let value: u32 = line
        .split(':')
        .nth(1)
        .and_then(|v| v.trim().trim_end_matches(';').parse().ok())
        .expect("--z-overlay is a number");
    assert!(
        value > 100,
        "--z-overlay ({value}) must clear a z-50 header and the z-100 grain"
    );
}

#[test]
fn the_shipped_menu_puts_the_layer_on_its_positioner() {
    let positioner = DROPDOWN
        .lines()
        .find(|l| l.contains("isolate"))
        .expect("the menu's positioner line");
    assert!(
        positioner.contains("z-(--z-overlay)"),
        "the positioner must carry `z-(--z-overlay)`: {positioner}"
    );
    assert!(
        !DROPDOWN.contains("\"z-50 max-h"),
        "a z-index on the popup inside the positioner does nothing"
    );
}

#[test]
fn the_language_picker_list_sits_above_the_navbar() {
    assert!(
        PICKER_TSX.contains("Menu.Positioner className={styles.positioner}"),
        "the picker's positioner must take the layer class"
    );
    assert!(
        rule(PICKER_CSS, ".positioner").contains("var(--z-overlay"),
        ".positioner must use --z-overlay"
    );
    assert!(
        !rule(PICKER_CSS, ".menu").contains("z-index"),
        "a z-index on .menu does nothing: the popup is not positioned"
    );
}
