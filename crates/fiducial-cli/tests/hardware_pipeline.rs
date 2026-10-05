//! End-to-end test for the `hardware` capability.
//!
//! Scaffolds a real product, installs `hardware`, and drives `fid derive`
//! through `fid-hardware` — the path a user takes. Asserts that the seed
//! solves, that what it solves follows the declaration, that a part which
//! cannot fit fails naming it, and — the reason input tracking exists — that
//! a redrawn SVG the declaration *reads* fails `--check` even though no output
//! was touched.

use std::{path::Path, process::Command};

fn fid(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fid"))
        .args(args)
        .current_dir(dir)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("failed to invoke fid")
}

fn text(o: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

fn scaffold(tmp: &Path) -> std::path::PathBuf {
    let out = fid(tmp, &["new", "demo"]);
    assert!(out.status.success(), "fid new failed: {}", text(&out));
    let root = tmp.join("demo");
    let out = fid(&root, &["add", "capability", "hardware"]);
    assert!(
        out.status.success(),
        "fid add capability hardware failed: {}",
        text(&out)
    );
    root
}

fn layout(root: &Path) -> serde_json::Value {
    serde_json::from_str(
        &std::fs::read_to_string(root.join("hardware/generated/layout.json")).unwrap(),
    )
    .unwrap()
}

fn edit(root: &Path, file: &str, from: &str, to: &str) {
    let p = root.join(file);
    let s = std::fs::read_to_string(&p).unwrap();
    assert!(s.contains(from), "{file} does not contain {from:?}");
    std::fs::write(&p, s.replacen(from, to, 1)).unwrap();
}

#[test]
fn the_seed_solves_and_every_output_is_written() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    for f in ["layout.json", "bom.csv", "assembly.md", "board.kicad_pcb"] {
        assert!(
            root.join("hardware/generated").join(f).is_file(),
            "{f} missing"
        );
    }
    let l = layout(&root);
    // The window part sets the scale: the region grew until it met the part.
    assert_eq!(l["scale"]["set_by"][0]["rule"], "cover");
    // Screws are derived, and land in the BOM without anyone listing them.
    let bom = std::fs::read_to_string(root.join("hardware/generated/bom.csv")).unwrap();
    assert!(bom.contains("screw-M3x"), "{bom}");
    assert!(bom.contains("gasket-lid"), "{bom}");
    // The board is sized from its one 18 mm part, far under the 100 mm ceiling.
    let side = l["board"]["body"]["size_mm"][0].as_f64().unwrap();
    assert!(side > 18.0 && side < 60.0, "board side {side}");
    let pcb = std::fs::read_to_string(root.join("hardware/generated/board.kicad_pcb")).unwrap();
    assert!(
        pcb.starts_with("(kicad_pcb")
            && pcb.contains("Edge.Cuts")
            && pcb.matches("np_thru_hole").count() == 4
    );
    let out = fid(&root, &["derive", "--check"]);
    assert!(out.status.success(), "{}", text(&out));
}

#[test]
fn a_bigger_window_part_makes_a_bigger_product() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(fid(&root, &["derive", "--pipeline", "hardware"])
        .status
        .success());
    let before = layout(&root)["scale"]["outline_width_mm"].as_f64().unwrap();
    edit(
        &root,
        "hardware/product.toml",
        "body_mm   = [60.0, 52.0, 3.0]",
        "body_mm   = [90.0, 78.0, 3.0]",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let after = layout(&root)["scale"]["outline_width_mm"].as_f64().unwrap();
    assert!(after > before * 1.4, "{before} -> {after}");
}

#[test]
fn a_part_that_cannot_fit_fails_naming_it() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    edit(
        &root,
        "hardware/product.toml",
        "body_mm = [70.0, 21.0, 21.0]",
        "body_mm = [400.0, 21.0, 21.0]",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    let t = text(&out);
    assert!(
        t.contains("`cell` socket") && t.contains("do not fit"),
        "{t}"
    );
}

#[test]
fn a_board_too_big_for_the_chamfered_case_fits_once_its_corners_may_be_cut() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let p = root.join("hardware/product.toml");
    let s = std::fs::read_to_string(&p).unwrap();
    // No cell, no screw bosses: the board alone against the octagon's chamfers.
    let s = s[..s.find("[[part]]\nid      = \"cell\"").unwrap()]
        .replace(
            "max_spacing_mm = 80.0",
            "max_spacing_mm = 80.0\nclosure = \"press-fit\"",
        )
        .replace(
            "max_mm = [100.0, 100.0]",
            "max_mm  = [120.0, 120.0]\nsize_mm = [116.0, 90.0]",
        );
    std::fs::write(&p, &s).unwrap();
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("fits nowhere"), "{}", text(&out));

    edit(
        &root,
        "hardware/product.toml",
        "size_mm = [116.0, 90.0]",
        "size_mm = [116.0, 90.0]\ncut_mm  = 10.0",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    assert_eq!(
        l["board"]["body"]["size_mm"],
        serde_json::json!([116.0, 90.0])
    );
    let outline: Vec<(f64, f64)> = l["board"]["outline_mm"]
        .as_array()
        .expect("a cut board has an outline")
        .iter()
        .map(|v| (v[0].as_f64().unwrap(), v[1].as_f64().unwrap()))
        .collect();
    assert_eq!(outline.len(), 8, "four corners cut: {outline:?}");
    let inside = |(x, y): (f64, f64)| {
        let mut c = false;
        for i in 0..outline.len() {
            let (a, b) = (outline[i], outline[(i + 1) % outline.len()]);
            if (a.1 > y) != (b.1 > y) && x < a.0 + (y - a.1) * (b.0 - a.0) / (b.1 - a.1) {
                c = !c;
            }
        }
        c
    };
    let (cx, cy) = (
        l["board"]["body"]["centre_mm"][0].as_f64().unwrap(),
        l["board"]["body"]["centre_mm"][1].as_f64().unwrap(),
    );
    // The corners are gone; the holes moved in with them.
    assert!(!inside((cx - 57.9, cy - 44.9)));
    for h in l["board"]["holes_mm"].as_array().unwrap() {
        assert!(
            inside((h[0].as_f64().unwrap(), h[1].as_f64().unwrap())),
            "{h}"
        );
    }
    assert!(l["why"].as_array().unwrap().iter().any(|w| w
        .as_str()
        .unwrap()
        .contains("board cut to the case at 4 corner(s)")));
    let pcb = std::fs::read_to_string(root.join("hardware/generated/board.kicad_pcb")).unwrap();
    assert!(
        pcb.contains("gr_poly") && !pcb.contains("(gr_rect"),
        "the board edge is the outline"
    );

    // A part pinned in a cut corner is refused, naming the corner.
    add_board_part(
        &root,
        "id = \"a\"\nname = \"A\"\nbody_mm = [3.0, 3.0, 1.0]\nat_mm = [2.0, 2.0]",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("cut to follow the case"),
        "{}",
        text(&out)
    );
}

#[test]
fn an_upstream_svg_that_moved_fails_check_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(fid(&root, &["derive", "--pipeline", "hardware"])
        .status
        .success());
    assert!(fid(&root, &["derive", "--check"]).status.success());

    // Redraw the outline and re-derive nothing. Every output still matches its
    // recorded hash, so an output-only gate passes this.
    edit(&root, "hardware/outline.svg", "H90 L100,10", "H91 L100,10");
    let out = fid(&root, &["derive", "--check"]);
    assert!(!out.status.success(), "check passed on a moved input");
    let t = text(&out);
    assert!(
        t.contains("hardware/outline.svg") && t.contains("changed since `hardware` last ran"),
        "{t}"
    );

    // Re-deriving records the new input, and the redraw is visible in the
    // layout as geometry, not only as a hash.
    assert!(fid(&root, &["derive", "--pipeline", "hardware"])
        .status
        .success());
    assert!(fid(&root, &["derive", "--check"]).status.success());
    let hi = &layout(&root)["shapes_read"]["outline"]["bounds_svg_units"][1];
    assert_eq!(hi[0].as_f64().unwrap(), 100.0);
}

#[test]
fn a_curve_in_the_outline_is_refused_not_flattened() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    edit(
        &root,
        "hardware/outline.svg",
        "H90 L100,10",
        "H90 Q100,0 100,10",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("`Q` is not supported"),
        "{}",
        text(&out)
    );
}

#[test]
fn a_priced_bom_over_its_ceiling_fails_derive() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    edit(
        &root,
        "hardware/product.toml",
        "status  = \"assumed\"\n\n[[part]]\nid      = \"cell\"",
        "status  = \"assumed\"\nunit_cost = 99.0\n\n[[part]]\nid      = \"cell\"",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("over the 30 ceiling"), "{}", text(&out));
}

#[test]
fn adding_the_capability_keeps_a_declaration_the_product_already_wrote() {
    let tmp = tempfile::tempdir().unwrap();
    let out = fid(tmp.path(), &["new", "demo"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("demo");
    std::fs::create_dir_all(root.join("hardware")).unwrap();
    let authored = "# authored before the capability existed\n[product]\nname = \"mine\"\n";
    std::fs::write(root.join("hardware/product.toml"), authored).unwrap();

    let out = fid(&root, &["add", "capability", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains("kept   hardware/product.toml"),
        "{}",
        text(&out)
    );
    assert_eq!(
        std::fs::read_to_string(root.join("hardware/product.toml")).unwrap(),
        authored
    );
    // The seed outline it did not have is still installed.
    assert!(root.join("hardware/outline.svg").is_file());
    // And the build script arrives runnable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(root.join("hardware/build.sh"))
            .unwrap()
            .permissions()
            .mode();
        assert!(
            mode & 0o111 != 0,
            "hardware/build.sh is not executable: {mode:o}"
        );
    }
}

/// A product that uses the second round of vocabulary: a panel on the lid
/// whose width sets the region's, parts under the board, wires, and a closure
/// chosen per test.
fn surface_product(closure: &str) -> String {
    format!(
        r#"
[product]
name = "node"

[outline]
svg   = "hardware/outline.svg"
layer = "outline"

[regions.window]
svg   = "hardware/outline.svg"
layer = "window"

[case]
wall_mm  = 5.0
floor_mm = 3.0
lid_mm   = 6.0

[case.fasteners]
closure        = "{closure}"
size           = "M3"
count          = 4
max_spacing_mm = 120.0

[board]
max_mm = [100.0, 100.0]

[cost]
currency = "EUR"
ceiling  = 30.0

[[part]]
id      = "panel"
name    = "Panel on the lid"
place   = "window"
mount   = "surface"
body_mm = [52.0, 52.0, 2.5]

[[part]]
id      = "mcu"
name    = "Module"
place   = "board"
mount   = "smd"
body_mm = [18.0, 18.0, 3.0]

[[part]]
id      = "conn"
name    = "Connector"
place   = "board"
mount   = "tht"
body_mm = [6.0, 4.5, 6.0]

[[part]]
id      = "cap"
name    = "Supercapacitor"
place   = "under-board"
mount   = "socket"
shape   = "cylinder"
body_mm = [30.0, 16.0, 16.0]

[[wire]]
id   = "panel"
from = "panel"
to   = "conn"

[[wire]]
id   = "cap"
from = "cap"
to   = "conn"
"#
    )
}

fn derive_product(toml: &str) -> (tempfile::TempDir, std::path::PathBuf, std::process::Output) {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    std::fs::write(root.join("hardware/product.toml"), toml).unwrap();
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    (tmp, root, out)
}

#[test]
fn a_surface_part_sets_the_region_width_exactly() {
    let (_tmp, root, out) = derive_product(&surface_product("screws-back"));
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    // The seed window is 40 of the outline's 100 units wide, so a 52 mm panel
    // makes the outline exactly 130 mm — not rounded.
    assert_eq!(l["scale"]["outline_width_mm"].as_f64().unwrap(), 130.0);
    assert_eq!(l["scale"]["set_by"][0]["rule"], "width");
    let rp = &l["region_parts"][0];
    assert_eq!(rp["mount"], "surface");
    assert!(rp["pass_through_mm"].as_f64().unwrap() > 0.0);
    let bom = std::fs::read_to_string(root.join("hardware/generated/bom.csv")).unwrap();
    assert!(bom.contains("adhesive-panel"), "{bom}");
}

#[test]
fn parts_under_the_board_raise_it_and_are_soldered_before_it() {
    let (_tmp, root, out) = derive_product(&surface_product("screws-back"));
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    let standoff = l["board"]["standoff_mm"].as_f64().unwrap();
    assert!(
        standoff >= 16.0,
        "board must clear the 16 mm can, standoff {standoff}"
    );
    assert_eq!(l["sockets"][0]["level"], "under-board");
    let steps = std::fs::read_to_string(root.join("hardware/generated/assembly.md")).unwrap();
    let solder = steps.find("Solder `cap`").expect(&steps);
    let seat = steps.find("Seat the board").expect(&steps);
    assert!(
        solder < seat,
        "the wire under the board must be soldered first:\n{steps}"
    );
}

#[test]
fn a_wire_over_the_tallest_part_still_runs_under_the_lid() {
    // The connector stands 6 mm on the board, so "over the tallest part"
    // (28.0) would put a 1.2 mm wire into a lid whose underside is 28.3.
    let (_tmp, root, out) = derive_product(&surface_product("screws-back"));
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    let base = l["case"]["base_height_mm"].as_f64().unwrap();
    let lid = base + l["case"]["lid_mm"].as_f64().unwrap();
    for w in l["wires"].as_array().unwrap() {
        for core in w["cores_mm"].as_array().unwrap() {
            for p in core["path_mm"].as_array().unwrap() {
                let z = p[2].as_f64().unwrap();
                assert!(
                    z <= base - 0.6 + 1e-9 || z >= lid - 1e-9,
                    "wire `{}` runs at {z} mm, inside the lid ({base}–{lid} mm)",
                    w["id"]
                );
            }
        }
    }
}

#[test]
fn a_wire_to_the_lid_carries_a_service_loop_into_the_bom() {
    let (_tmp, root, out) = derive_product(&surface_product("screws-back"));
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    let w = l["wires"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["id"] == "panel")
        .unwrap()
        .clone();
    assert_eq!(w["lid_mounted"], true);
    let (routed, length) = (
        w["routed_mm"].as_f64().unwrap(),
        w["length_mm"].as_f64().unwrap(),
    );
    assert!(
        length >= routed + 60.0,
        "service loop missing: routed {routed}, cut {length}"
    );
    let bom = std::fs::read_to_string(root.join("hardware/generated/bom.csv")).unwrap();
    assert!(
        bom.contains(&format!("{length} mm (panel → conn)")),
        "{bom}"
    );
}

#[test]
fn screws_from_the_back_land_on_a_standard_length_with_enough_bite() {
    let (_tmp, root, out) = derive_product(&surface_product("screws-back"));
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    let s = &l["case"]["screws"];
    let (len, h, cb) = (
        s["length_mm"].as_f64().unwrap(),
        l["case"]["base_height_mm"].as_f64().unwrap(),
        s["counterbore_mm"].as_f64().unwrap(),
    );
    let bite = len - (h - cb);
    assert!(
        (4.5..=5.0 + 1e-9).contains(&bite),
        "bite {bite} mm: 1.5 × M3 minimum, 1 mm short of the lid face maximum"
    );
    assert_eq!(s["at_mm"].as_array().unwrap().len(), 4);
    let bom = std::fs::read_to_string(root.join("hardware/generated/bom.csv")).unwrap();
    assert!(bom.contains("from underneath"), "{bom}");
}

#[test]
fn a_press_fit_lid_has_no_screws_and_says_what_it_trades() {
    let (_tmp, root, out) = derive_product(&surface_product("press-fit"));
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    assert!(l["case"]["screws"]["at_mm"].as_array().unwrap().is_empty());
    assert!(l["case"]["press_fit"]["interference_mm"].as_f64().unwrap() > 0.0);
    let bom = std::fs::read_to_string(root.join("hardware/generated/bom.csv")).unwrap();
    assert!(!bom.contains("screw-M3"), "{bom}");
    assert!(l["why"]
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w.as_str().unwrap().contains("friction alone")));
}

#[test]
fn a_wire_to_an_undeclared_part_fails_naming_it() {
    let toml = surface_product("screws-back")
        .replace("to   = \"conn\"\n\n[[wire]]", "to   = \"nope\"\n\n[[wire]]");
    let (_tmp, _root, out) = derive_product(&toml);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("part `nope`, which is not declared"),
        "{}",
        text(&out)
    );
}

#[test]
fn a_scoped_upgrade_refreshes_one_capability_and_its_instructions_only() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let out = fid(&root, &["add", "capability", "brand"]);
    assert!(out.status.success(), "{}", text(&out));
    // Stale instructions for hardware, and a hand-marked file belonging to
    // another installed capability, which a scoped upgrade must not touch.
    std::fs::write(root.join(".fiducial/skills/hardware.md"), "stale\n").unwrap();
    let other = std::fs::read_dir(root.join(".fiducial/skills"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| !p.ends_with("hardware.md"))
        .expect("a second capability's instructions");
    std::fs::write(&other, "mine\n").unwrap();

    let out = fid(&root, &["upgrade", "--capability", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let skill = std::fs::read_to_string(root.join(".fiducial/skills/hardware.md")).unwrap();
    assert!(
        skill.contains("fiducial:hardware"),
        "instructions not refreshed"
    );
    // The Claude file stays a pointer: the content lives once.
    let pointer = std::fs::read_to_string(root.join(".claude/skills/hardware.md")).unwrap();
    assert!(
        pointer.contains("Read `.fiducial/skills/hardware.md`")
            && !pointer.contains("## The model"),
        "{pointer}"
    );
    assert_eq!(
        std::fs::read_to_string(&other).unwrap(),
        "mine\n",
        "a scoped upgrade touched another capability"
    );
    assert!(text(&out).contains("skipped (scoped upgrade)"));
}

#[test]
fn a_scoped_upgrade_refuses_a_capability_that_is_not_installed() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let out = fid(&root, &["upgrade", "--capability", "cms"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("`cms` is not installed"),
        "{}",
        text(&out)
    );
}

#[test]
fn a_gasket_stands_proud_of_its_groove_so_the_lid_can_squeeze_it() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(fid(&root, &["derive", "--pipeline", "hardware"])
        .status
        .success());
    let s = &layout(&root)["case"]["seal"];
    let (h, d) = (
        s["gasket_height_mm"].as_f64().unwrap(),
        s["depth_mm"].as_f64().unwrap(),
    );
    assert!(
        h > d,
        "gasket {h} mm in a {d} mm groove: it must stand proud to seal"
    );
    let squeezed = s["gasket_compressed_mm"].as_f64().unwrap();
    assert!(
        (squeezed - h * 0.75).abs() < 1e-3,
        "compressed to {squeezed}, declared 25%"
    );
}

#[test]
fn a_gasket_shorter_than_its_groove_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    // A declared tongue this deep leaves a gasket sunk below the rim.
    edit(
        &root,
        "hardware/product.toml",
        "compression       = 0.25",
        "compression       = 0.25\ntongue_mm         = 2.0",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("it seals nothing"), "{}", text(&out));
}

#[test]
fn a_zone_grows_to_its_contents_along_its_own_axis() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    edit(
        &root,
        "hardware/product.toml",
        "max_mm = [100.0, 100.0]",
        "max_mm = [100.0, 100.0]\nzones  = { rf = \"top\" }",
    );
    edit(
        &root,
        "hardware/product.toml",
        "body_mm = [18.0, 18.0, 3.0]",
        "body_mm = [18.0, 40.0, 3.0]\nzone    = \"rf\"\nkeepout_mm = 3.0\nfaces   = \"top\"",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    let size = &l["board"]["body"]["size_mm"];
    let (w, h) = (size[0].as_f64().unwrap(), size[1].as_f64().unwrap());
    // 40 mm of body + 3 mm keep-out + 1 mm: the top band needs 44 mm.
    assert!(h >= 44.0, "board {w} × {h}");
    // The part stays inside its band, keep-out included.
    let mcu = &l["board"]["illustrative_placement"][0];
    let top = l["board"]["body"]["centre_mm"][1].as_f64().unwrap() + h / 2.0;
    let ko = &mcu["keepout"];
    let ko_top = ko["centre_mm"][1].as_f64().unwrap() + ko["size_mm"][1].as_f64().unwrap() / 2.0;
    assert!(
        (ko_top - top).abs() < 1e-6,
        "keep-out runs to the top edge: {ko_top} vs {top}"
    );
}

#[test]
fn wire_pads_become_copper_carrying_their_nets() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let p = root.join("hardware/product.toml");
    let mut s = std::fs::read_to_string(&p).unwrap();
    s.push_str(
        "\n[[part]]\nid       = \"vin\"\nname     = \"Supply wire pads\"\nplace    = \"board\"\nmount    = \"pads\"\n\
         nets     = [\"VIN+\", \"VIN-\"]\npad_mm   = [2.5, 3.5]\npitch_mm = 4.0\nbody_mm  = [7.5, 4.5, 0.1]\nstatus   = \"decided\"\n",
    );
    std::fs::write(&p, s).unwrap();
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let pcb = std::fs::read_to_string(root.join("hardware/generated/board.kicad_pcb")).unwrap();
    assert!(
        pcb.contains("(net 1 \"VIN+\")") && pcb.contains("(net 2 \"VIN-\")"),
        "{pcb}"
    );
    assert!(pcb.contains("fid:WirePads_1x2"), "{pcb}");
    assert_eq!(
        pcb.matches("(net 1 \"VIN+\"))").count(),
        1,
        "one pad per net: {pcb}"
    );
}

#[test]
fn a_socket_reaches_its_near_side_when_the_board_can_move_aside() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    edit(
        &root,
        "hardware/product.toml",
        "shape   = \"cylinder\"",
        "shape   = \"cylinder\"\nnear    = \"bottom\"",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    let ys: Vec<f64> = l["case"]["outline_mm"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p[1].as_f64().unwrap())
        .collect();
    let (lo, hi) = (
        ys.iter().cloned().fold(f64::MAX, f64::min),
        ys.iter().cloned().fold(f64::MIN, f64::max),
    );
    let cell = l["sockets"][0]["body"]["centre_mm"][1].as_f64().unwrap();
    let board = l["board"]["body"]["centre_mm"][1].as_f64().unwrap();
    assert!(
        cell < lo + (hi - lo) / 3.0,
        "cell at y {cell} in {lo}..{hi}"
    );
    assert!(
        board > cell,
        "the board moves aside, above it: board {board}, cell {cell}"
    );
}

#[test]
fn an_upgrade_delivers_a_file_the_capability_gained_after_install() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    // Make this product look like it installed `hardware` before `parts.py`
    // existed: no file, no lock entry.
    let lock_path = root.join("fiducial.lock");
    let lock = std::fs::read_to_string(&lock_path).unwrap();
    let start = lock
        .find("[templates.\"hardware/parts.py\"]")
        .expect("parts.py is tracked");
    let end = lock[start + 1..]
        .find("\n[")
        .map(|i| start + 1 + i + 1)
        .unwrap_or(lock.len());
    std::fs::write(&lock_path, format!("{}{}", &lock[..start], &lock[end..])).unwrap();
    std::fs::remove_file(root.join("hardware/parts.py")).unwrap();

    let out = fid(&root, &["upgrade", "--capability", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains("added: hardware/parts.py (new in `hardware`)"),
        "{}",
        text(&out)
    );
    assert!(root.join("hardware/parts.py").exists());
    assert!(std::fs::read_to_string(&lock_path)
        .unwrap()
        .contains("[templates.\"hardware/parts.py\"]"));

    // A file of the product's own at a path the capability later ships is
    // not the platform's to overwrite.
    let lock = std::fs::read_to_string(&lock_path).unwrap();
    let start = lock.find("[templates.\"hardware/parts.py\"]").unwrap();
    let end = lock[start + 1..]
        .find("\n[")
        .map(|i| start + 1 + i + 1)
        .unwrap_or(lock.len());
    std::fs::write(&lock_path, format!("{}{}", &lock[..start], &lock[end..])).unwrap();
    std::fs::write(root.join("hardware/parts.py"), "# mine\n").unwrap();
    let out = fid(&root, &["upgrade", "--capability", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains("hardware/parts.py: exists on disk but is untracked"),
        "{}",
        text(&out)
    );
    assert_eq!(
        std::fs::read_to_string(root.join("hardware/parts.py")).unwrap(),
        "# mine\n"
    );
}

/// A two-symbol library, as `parts.py sync` would vendor it.
const SYMBOLS: &str = r#"(kicad_symbol_lib (version 20220914)
  (symbol "Device:R" (property "Reference" "R" (at 0 0 0))
    (symbol "R_0_1" (rectangle (start -1 2.5) (end 1 -2.5)))
    (symbol "R_1_1"
      (pin passive line (at 0 3.81 270) (length 1.27) (name "~" (effects)) (number "1" (effects)))
      (pin passive line (at 0 -3.81 90) (length 1.27) (name "~" (effects)) (number "2" (effects)))))
  (symbol "Lib:M" (property "Reference" "U" (at 0 0 0))
    (symbol "M_0_1" (rectangle (start -5 5) (end 5 -5)))
    (symbol "M_1_1"
      (pin power_in line (at -7.62 2.54 0) (length 2.54) (name "VDD" (effects)) (number "1" (effects)))
      (pin power_in line (at -7.62 0 0) (length 2.54) (name "GND" (effects)) (number "2" (effects)))
      (pin power_in line (at -7.62 -2.54 0) (length 2.54) (name "GND" (effects)) (number "3" (effects)))
      (pin input line (at 7.62 0 180) (length 2.54) (name "EN" (effects)) (number "4" (effects))))))
"#;

fn with_symbols(root: &Path, mcu_pins: &str) {
    std::fs::create_dir_all(root.join("hardware/lib")).unwrap();
    std::fs::write(root.join("hardware/lib/symbols.kicad_sym"), SYMBOLS).unwrap();
    edit(
        root,
        "hardware/product.toml",
        "body_mm = [18.0, 18.0, 3.0]",
        &format!("body_mm = [18.0, 18.0, 3.0]\nsymbol  = \"Lib:M\"\npins    = {mcu_pins}"),
    );
    let p = root.join("hardware/product.toml");
    let mut s = std::fs::read_to_string(&p).unwrap();
    s.push_str(
        "\n[[part]]\nid      = \"r-en\"\nname    = \"Enable pull-up\"\nplace   = \"board\"\nmount   = \"smd\"\n\
         body_mm = [1.6, 0.8, 0.45]\nsymbol  = \"Device:R\"\nvalue   = \"100k\"\nnear    = \"mcu.EN\"\npins    = { 1 = \"VCC\", 2 = \"EN\" }\n",
    );
    std::fs::write(&p, s).unwrap();
}

#[test]
fn declared_pins_become_nets_on_the_board_and_labels_in_the_schematic() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    with_symbols(&root, "{ VDD = \"VCC\", GND = \"GND\", EN = \"EN\" }");
    // A net needs two ends: give GND its second.
    let p = root.join("hardware/product.toml");
    let s = std::fs::read_to_string(&p).unwrap();
    std::fs::write(&p, s +"\n[[part]]\nid = \"r-gnd\"\nname = \"Ground tie\"\nplace = \"board\"\nmount = \"smd\"\nbody_mm = [1.6, 0.8, 0.45]\nsymbol = \"Device:R\"\nvalue = \"0R\"\npins = { 1 = \"GND\", 2 = \"VCC\" }\n").unwrap();
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    let mcu = l["board"]["illustrative_placement"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["part"] == "mcu")
        .unwrap()
        .clone();
    // Both GND pins, by name.
    assert_eq!(mcu["pin_nets"]["2"], "GND");
    assert_eq!(mcu["pin_nets"]["3"], "GND");
    assert_eq!(mcu["ref"], "U1");
    let sch = std::fs::read_to_string(root.join("hardware/generated/board.kicad_sch")).unwrap();
    assert!(
        sch.contains("(lib_id \"Device:R\")") && sch.contains("(lib_id \"Lib:M\")"),
        "{sch}"
    );
    assert_eq!(
        sch.matches("(label \"GND\"").count(),
        3,
        "two mcu pins and the tie"
    );
    assert!(sch.contains("(property \"Value\" \"100k\""));
    // Derived, so byte-stable: a second derive changes nothing.
    let out = fid(&root, &["derive", "--check", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
}

#[test]
fn a_pin_the_symbol_does_not_have_fails_listing_the_pins_it_does() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    with_symbols(&root, "{ VDD = \"VCC\", RESET = \"EN\" }");
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    let t = text(&out);
    assert!(
        t.contains("pin `RESET` is not a pin of Lib:M") && t.contains("4 EN"),
        "{t}"
    );
}

#[test]
fn a_net_with_one_end_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    with_symbols(&root, "{ VDD = \"VCC\", GND = \"GND\", EN = \"EN\" }");
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    let t = text(&out);
    assert!(
        t.contains("net `GND` reaches only `mcu.") && t.contains("two ends"),
        "{t}"
    );
}

#[test]
fn a_wire_runs_from_a_named_lead_to_a_named_pad() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    edit(
        &root,
        "hardware/product.toml",
        "body_mm = [70.0, 21.0, 21.0]",
        "body_mm = [50.0, 21.0, 21.0]\nleads   = [\"+\", \"-\"]\npitch_mm = 8.0\nlead_mm = 4.0",
    );
    let p = root.join("hardware/product.toml");
    let mut s = std::fs::read_to_string(&p).unwrap();
    s.push_str(
        "\n[[part]]\nid = \"cell-pads\"\nname = \"Cell pads\"\nplace = \"board\"\nmount = \"pads\"\nnear = \"cell\"\n\
         nets = [\"BAT+\", \"BAT-\"]\npad_mm = [2.0, 3.0]\npitch_mm = 8.0\nbody_mm = [10.0, 4.0, 0.1]\n\
         \n[[wire]]\nid = \"bat-plus\"\nfrom = \"cell.+\"\nto = \"cell-pads.1\"\ncores = 1\n",
    );
    std::fs::write(&p, s).unwrap();
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    let lead = &l["sockets"][0]["leads"][0];
    assert_eq!(lead["pin"], "+");
    let pad = l["board"]["illustrative_placement"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["part"] == "cell-pads")
        .unwrap()["pads_at"][0]["at_mm"]
        .clone();
    let w = &l["wires"][0];
    let path = w["path_mm"].as_array().unwrap();
    assert_eq!(
        path.first().unwrap()[0],
        lead["at_mm"][0],
        "starts at the lead's tip"
    );
    assert_eq!(path.last().unwrap()[0], pad[0], "ends on the pad");
    // Low, not up to the lid and back; dressed square: every run along one
    // axis, and only a few of them.
    let z = |q: &serde_json::Value| q[2].as_f64().unwrap();
    assert!(path.iter().all(|q| z(q) < 20.0), "{path:?}");
    for s in path.windows(2) {
        let moved = (0..3)
            .filter(|&k| (s[0][k].as_f64().unwrap() - s[1][k].as_f64().unwrap()).abs() > 1e-6)
            .count();
        assert!(moved <= 1, "a diagonal run: {s:?}");
    }
    assert!(path.len() <= 6, "{path:?}");
}

#[test]
fn a_two_core_wire_lands_each_core_on_its_own_pad_without_crossing() {
    let (_tmp, root, out) = derive_product(&surface_product("screws-back").replace(
        "[[wire]]\nid   = \"panel\"\nfrom = \"panel\"\nto   = \"conn\"",
        "[[part]]\nid = \"panel-pads\"\nname = \"Panel pads\"\nplace = \"board\"\nmount = \"pads\"\n\
         nets = [\"PV+\", \"GND\"]\npad_mm = [2.0, 3.0]\npitch_mm = 4.0\nbody_mm = [8.0, 4.0, 0.1]\n\n\
         [[wire]]\nid   = \"panel\"\nfrom = \"panel\"\nto   = \"panel-pads\"",
    ));
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    let w = l["wires"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["id"] == "panel")
        .unwrap()
        .clone();
    let cores = w["cores_mm"].as_array().unwrap();
    assert_eq!(cores.len(), 2);
    let pads = l["board"]["illustrative_placement"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["part"] == "panel-pads")
        .unwrap()["pads_at"]
        .as_array()
        .unwrap()
        .clone();
    let xy = |v: &serde_json::Value| (v[0].as_f64().unwrap(), v[1].as_f64().unwrap());
    let mut landed: Vec<(f64, f64)> = cores
        .iter()
        .map(|c| xy(c["path_mm"].as_array().unwrap().last().unwrap()))
        .collect();
    let mut want: Vec<(f64, f64)> = pads.iter().map(|p| xy(&p["at_mm"])).collect();
    landed.sort_by(|a, b| a.partial_cmp(b).unwrap());
    want.sort_by(|a, b| a.partial_cmp(b).unwrap());
    for (a, b) in landed.iter().zip(&want) {
        assert!(
            (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3,
            "a core on each pad: {landed:?} vs {want:?}"
        );
    }
    // In plan, the two cores never cross.
    let seg = |c: &serde_json::Value| -> Vec<(f64, f64)> {
        c["path_mm"].as_array().unwrap().iter().map(xy).collect()
    };
    let (a, b) = (seg(&cores[0]), seg(&cores[1]));
    let d = |p: (f64, f64), q: (f64, f64), r: (f64, f64)| {
        (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0)
    };
    for x in a.windows(2) {
        for y in b.windows(2) {
            let crossing = d(x[0], x[1], y[0]) * d(x[0], x[1], y[1]) < -1e-9
                && d(y[0], y[1], x[0]) * d(y[0], y[1], x[1]) < -1e-9;
            assert!(!crossing, "cores cross: {a:?} / {b:?}");
        }
    }
    // Each core carries its pad's net.
    let nets: Vec<&str> = cores.iter().map(|c| c["net"].as_str().unwrap()).collect();
    assert!(nets.contains(&"PV+") && nets.contains(&"GND"), "{nets:?}");
}

#[test]
fn a_roof_mark_a_vent_in_it_and_a_hanging_lug() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    // A house in the outline: walls, and a roof above its full-width span.
    std::fs::write(
        root.join("hardware/outline.svg"),
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 120">
  <g data-layer="outline"><path d="M10,0 H90 L100,10 V110 L90,120 H10 L0,110 V10 Z"/></g>
  <g data-layer="house"><path d="M30,100 H70 V60 L50,40 L30,60 Z"/></g>
</svg>"#,
    )
    .unwrap();
    let p = root.join("hardware/product.toml");
    let s = std::fs::read_to_string(&p)
        .unwrap()
        .replace("[regions.window]\nsvg   = \"hardware/outline.svg\"\nlayer = \"window\"", "[regions.house]\nsvg   = \"hardware/outline.svg\"\nlayer = \"house\"")
        .replace(
            "place     = \"window\"\nmount     = \"window\"\nbody_mm   = [60.0, 52.0, 3.0]",
            "place     = \"house\"\nmount     = \"surface\"\nbody_mm   = [40.0, 40.0, 2.5]",
        )
        .replace(
            "[case.fasteners]",
            "[[case.mark]]\nid = \"roof\"\nregion = \"house\"\non = \"display\"\n\n[case.hang]\nat = \"top\"\n\n[case.fasteners]",
        );
    std::fs::write(
        &p,
        s + "\n[[part]]\nid = \"vent\"\nname = \"Vent\"\nplace = \"roof\"\nmount = \"vent\"\nnear = \"display\"\nbody_mm = [8.0, 8.0, 0.3]\npass_through_mm = 3.0\n",
    )
    .unwrap();
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    let roof = l["case"]["marks"][0]["polygon_mm"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(
        roof.len(),
        3,
        "the roof is the triangle above the walls: {roof:?}"
    );
    let panel = l["region_parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["part"] == "display")
        .unwrap()
        .clone();
    let panel_top = panel["body"]["centre_mm"][1].as_f64().unwrap()
        + panel["body"]["size_mm"][1].as_f64().unwrap() / 2.0;
    let base = roof
        .iter()
        .map(|v| v[1].as_f64().unwrap())
        .fold(f64::MAX, f64::min);
    assert!(
        (base - panel_top).abs() < 1e-3,
        "the roof sits on the panel's top edge: {base} vs {panel_top}"
    );
    let vent = l["region_parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["part"] == "vent")
        .unwrap()
        .clone();
    let vy = vent["body"]["centre_mm"][1].as_f64().unwrap();
    assert!(
        vy > panel_top,
        "the vent is in the roof, above the panel: {vy} vs {panel_top}"
    );
    // Hanging by default is two holes through a solid tip: one face-on, one
    // across for a nail from the side, both below the outline's highest point.
    let hang = &l["case"]["hang"];
    assert_eq!(hang["kind"], "holes");
    let through = hang["through"]["at_mm"][1].as_f64().unwrap();
    let across = hang["across"]["y_mm"].as_f64().unwrap();
    assert!(across < through, "{hang}");
    assert!(
        hang["plug_y_mm"].as_f64().unwrap() < across,
        "the cavity stops below both holes: {hang}"
    );
    // Sealed to a board part, the vent stays where its own body fits the
    // roof: its chimney's ring is on the board below, not in the roof, and
    // the board reaches under it.
    std::fs::write(
        &p,
        std::fs::read_to_string(&p).unwrap().replace(
            "near = \"display\"\n",
            "near = \"display\"\nseals_to = \"gas\"\nmembrane = \"outside\"\n",
        ),
    )
    .unwrap();
    add_board_part(
        &root,
        "id = \"gas\"\nname = \"Gas sensor\"\nbody_mm = [3.0, 3.0, 1.0]",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    let sealed = l["region_parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["part"] == "vent")
        .unwrap();
    assert_eq!(sealed["body"]["centre_mm"], vent["body"]["centre_mm"]);
    assert_eq!(l["chimneys"][0]["part"], "gas");
}

#[test]
fn a_part_lying_beside_the_board_solders_into_plated_holes_and_the_board_snaps_in() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    edit(
        &root,
        "hardware/product.toml",
        "body_mm = [70.0, 21.0, 21.0]",
        "body_mm = [22.0, 12.5, 12.5]\nnear    = \"bottom\"\nleads   = [\"+\", \"-\"]\npitch_mm = 5.5\nlead_mm = 3.0",
    );
    edit(
        &root,
        "hardware/product.toml",
        "[board]",
        "[board]\nmount = \"snap\"",
    );
    let p = root.join("hardware/product.toml");
    let mut s = std::fs::read_to_string(&p).unwrap();
    s.push_str(
        "\n[[part]]\nid = \"cell-pads\"\nname = \"Cell holes\"\nplace = \"board\"\nmount = \"pads\"\n\
         follows = \"cell\"\ndrill_mm = 1.0\nnets = [\"BAT+\", \"BAT-\"]\npad_mm = [1.8, 1.8]\nbody_mm = [10.0, 4.0, 0.1]\n",
    );
    std::fs::write(&p, s).unwrap();
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    assert!(
        l["wires"].as_array().is_none_or(|w| w.is_empty()),
        "no wire: {}",
        l["wires"]
    );
    let leads = l["sockets"][0]["leads"].as_array().unwrap().clone();
    let pads = l["board"]["illustrative_placement"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["part"] == "cell-pads")
        .unwrap()["pads_at"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(pads.len(), 2);
    for (lead, pad) in leads.iter().zip(&pads) {
        assert_eq!(
            lead["hole_mm"][0], pad["at_mm"][0],
            "a hole under each lead: {lead} {pad}"
        );
    }
    let pcb = std::fs::read_to_string(root.join("hardware/generated/board.kicad_pcb")).unwrap();
    assert!(
        pcb.contains("thru_hole circle") && pcb.contains("(drill 1"),
        "plated holes"
    );
    assert_eq!(l["board"]["mount"], "snap");
}

#[test]
fn routing_rules_below_a_fab_process_are_refused_and_a_datasheet_reaches_the_bom() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    edit(
        &root,
        "hardware/product.toml",
        "[board]",
        "[board]\nedge_mm = 1.0\ntrack_mm = 0.08",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("board.track_mm"), "{}", text(&out));
    edit(
        &root,
        "hardware/product.toml",
        "track_mm = 0.08",
        "track_mm = 0.15",
    );
    edit(
        &root,
        "hardware/product.toml",
        "status    = \"assumed\"\nsettle_by",
        "status    = \"assumed\"\ndatasheet = \"https://example.com/display.pdf\"\nsettle_by",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    assert_eq!(l["board"]["track_mm"], 0.15);
    let bom = std::fs::read_to_string(root.join("hardware/generated/bom.csv")).unwrap();
    assert!(bom.lines().next().unwrap().contains(",datasheet,"), "{bom}");
    assert!(bom.contains("https://example.com/display.pdf"), "{bom}");
    // Nothing sits in the perimeter strip: every part clears the edge by 1 mm.
    let (c, s) = (
        &l["board"]["body"]["centre_mm"],
        &l["board"]["body"]["size_mm"],
    );
    let x0 = c[0].as_f64().unwrap() - s[0].as_f64().unwrap() / 2.0;
    for p in l["board"]["illustrative_placement"].as_array().unwrap() {
        let px = p["body"]["centre_mm"][0].as_f64().unwrap()
            - p["body"]["size_mm"][0].as_f64().unwrap() / 2.0;
        assert!(
            px >= x0 + 1.0 - 1e-6,
            "{} at {px} inside the edge strip ({x0})",
            p["part"]
        );
    }
}

/// Board parts are placed by a constraint solver (spec 2026-10-01): the
/// rules are data in hardware/generated/placement-model.json, the answer is
/// hardware/generated/placement.json, stamped with the model's hash.
fn add_board_part(root: &Path, toml: &str) {
    let p = root.join("hardware/product.toml");
    let s = std::fs::read_to_string(&p).unwrap();
    std::fs::write(
        &p,
        format!(
            "{s}\n[[part]]\nplace = \"board\"\nmount = \"smd\"\nstatus = \"assumed\"\n{toml}\n"
        ),
    )
    .unwrap();
}

fn board_part<'a>(l: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    l["board"]["illustrative_placement"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["part"] == id)
        .unwrap_or_else(|| panic!("no board part {id}"))
}

#[test]
fn a_declared_position_and_turn_are_kept_and_two_that_collide_are_named() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    add_board_part(
        &root,
        "id = \"a\"\nname = \"A\"\nbody_mm = [4.0, 4.0, 1.0]\nat_mm = [15.0, 20.0]",
    );
    add_board_part(&root, "id = \"b\"\nname = \"B\"\nbody_mm = [6.0, 2.0, 1.0]\nat_mm = [16.0, 20.0]\nrotate_deg = 90.0");
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    let t = text(&out);
    assert!(
        t.contains("a: at_mm = [15, 20]") && t.contains("b: at_mm = [16, 20]"),
        "{t}"
    );
    assert!(root.join("hardware/build/placement-model.json").exists());
    edit(
        &root,
        "hardware/product.toml",
        "at_mm = [16.0, 20.0]",
        "at_mm = [28.0, 20.0]",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    let x0 = l["board"]["body"]["centre_mm"][0].as_f64().unwrap()
        - l["board"]["body"]["size_mm"][0].as_f64().unwrap() / 2.0;
    let b = board_part(&l, "b");
    assert_eq!(b["rotation_deg"], 90.0);
    assert!(
        (b["body"]["centre_mm"][0].as_f64().unwrap() - (x0 + 28.0)).abs() < 0.11,
        "{b}"
    );
    assert_eq!(b["body"]["size_mm"][0], 2.0);
}

#[test]
fn a_part_kept_away_keeps_its_distance() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    add_board_part(
        &root,
        "id = \"hot\"\nname = \"Hot\"\nbody_mm = [6.0, 6.0, 1.0]",
    );
    add_board_part(&root, "id = \"sense\"\nname = \"Sense\"\nbody_mm = [3.0, 3.0, 1.0]\nnear = \"hot\"\naway_from = [\"hot\"]\naway_mm = 8.0");
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    let rect = |id: &str| {
        let p = board_part(&l, id);
        let c = &p["body"]["centre_mm"];
        let s = &p["body"]["size_mm"];
        (
            c[0].as_f64().unwrap(),
            c[1].as_f64().unwrap(),
            s[0].as_f64().unwrap(),
            s[1].as_f64().unwrap(),
        )
    };
    let (hx, hy, hw, hh) = rect("hot");
    let (sx, sy, sw, sh) = rect("sense");
    let gap_x = (sx - hx).abs() - (hw + sw) / 2.0;
    let gap_y = (sy - hy).abs() - (hh + sh) / 2.0;
    // Kept away, and no further than it has to be: it is also asked to be near.
    assert!(gap_x.max(gap_y) >= 8.0 - 0.11, "gap {gap_x} × {gap_y}");
    assert!(gap_x.max(gap_y) <= 8.5, "gap {gap_x} × {gap_y}");
}

#[test]
fn the_committed_placement_is_reused_until_its_model_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let answer = std::fs::read_to_string(root.join("hardware/generated/placement.json")).unwrap();
    assert!(answer.contains("model_sha256"), "{answer}");
    // No solver at all: the stamped answer still serves the same model.
    let no_python = |root: &Path, args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_fid"))
            .args(args)
            .current_dir(root)
            .env("RUST_BACKTRACE", "0")
            .env("FID_PYTHON", "/nonexistent/python3")
            .output()
            .unwrap()
    };
    let out = no_python(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let out = no_python(&root, &["derive", "--check"]);
    assert!(out.status.success(), "{}", text(&out));
    // A changed model needs the solver, and says so.
    add_board_part(
        &root,
        "id = \"extra\"\nname = \"Extra\"\nbody_mm = [3.0, 3.0, 1.0]",
    );
    let out = no_python(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("ortools"), "{}", text(&out));
}

#[test]
fn a_vent_sealed_to_a_board_part_gets_a_chimney_and_the_part_sits_under_it() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let p = root.join("hardware/product.toml");
    let s = std::fs::read_to_string(&p).unwrap();
    let vent = "\n[[part]]\nid = \"vent\"\nname = \"Vent\"\nplace = \"lid\"\nmount = \"vent\"\nbody_mm = [10.0, 10.0, 0.3]\nstatus = \"assumed\"\nseals_to = \"gas\"\nmembrane = \"outside\"\n";
    std::fs::write(&p, format!("{s}{vent}")).unwrap();
    add_board_part(
        &root,
        "id = \"gas\"\nname = \"Gas sensor\"\nbody_mm = [3.0, 3.0, 1.0]",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    let ch = &l["chimneys"][0];
    assert_eq!(ch["part"], "gas");
    // The tube's foot is round the part; its top at the vent, at most 3 mm
    // off — it leans no further than prints unsupported.
    let g = board_part(&l, "gas");
    for k in 0..2 {
        let d = g["body"]["centre_mm"][k].as_f64().unwrap() - ch["centre_mm"][k].as_f64().unwrap();
        assert!(d.abs() < 1e-6, "{g} vs {ch}");
    }
    let lean = (ch["centre_mm"][0].as_f64().unwrap() - ch["top_mm"][0].as_f64().unwrap())
        .hypot(ch["centre_mm"][1].as_f64().unwrap() - ch["top_mm"][1].as_f64().unwrap());
    assert!(lean <= 3.0 + 0.1, "leans {lean} mm");
    assert!(ch["bore_mm"].as_f64().unwrap() > 3.0_f64.hypot(3.0));
    // The whole ring on the board.
    let (bc, bs) = (
        &l["board"]["body"]["centre_mm"],
        &l["board"]["body"]["size_mm"],
    );
    let r = ch["ring_mm"].as_f64().unwrap() / 2.0;
    for k in 0..2 {
        let (c, half) = (bc[k].as_f64().unwrap(), bs[k].as_f64().unwrap() / 2.0);
        let at = ch["centre_mm"][k].as_f64().unwrap();
        assert!(
            at - r >= c - half - 1e-6 && at + r <= c + half + 1e-6,
            "ring off the board: {ch}"
        );
    }
    // Nothing else on the board inside its ring.
    let r = ch["ring_mm"].as_f64().unwrap() / 2.0;
    for q in l["board"]["illustrative_placement"].as_array().unwrap() {
        if q["part"] == "gas" {
            continue;
        }
        let (c, s) = (&q["body"]["centre_mm"], &q["body"]["size_mm"]);
        let dx = ((c[0].as_f64().unwrap() - ch["centre_mm"][0].as_f64().unwrap()).abs()
            - s[0].as_f64().unwrap() / 2.0)
            .max(0.0);
        let dy = ((c[1].as_f64().unwrap() - ch["centre_mm"][1].as_f64().unwrap()).abs()
            - s[1].as_f64().unwrap() / 2.0)
            .max(0.0);
        assert!(dx.hypot(dy) >= r, "{} inside the ring", q["part"]);
    }
    let bom = std::fs::read_to_string(root.join("hardware/generated/bom.csv")).unwrap();
    assert!(bom.contains("chimney-ring-vent"), "{bom}");
    // Membrane outside: the bore need only clear the part (and its vias),
    // not the 10 mm patch; and the ring lands on top copper free of tracks.
    assert!(ch["bore_mm"].as_f64().unwrap() < 7.0, "{ch}");
    let pcb = std::fs::read_to_string(root.join("hardware/generated/board.kicad_pcb")).unwrap();
    assert_eq!(pcb.matches("chimney-land-vent").count(), 12);
    // Sealing to something that is not on the board is refused.
    edit(
        &root,
        "hardware/product.toml",
        "seals_to = \"gas\"",
        "seals_to = \"cell\"",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("not a board part"), "{}", text(&out));
}

/// A catalogue in jlcparts' columns, small enough to read: one right answer
/// for a 10 kΩ 0603 1 % basic resistor, and a decoy for each rule.
const CATALOGUE: &str = r#"lcsc,category,subcategory,mfr,package,manufacturer,basic,preferred,description,stock,price
25804,Resistors,Chip Resistor - Surface Mount,0603WAF1002T5E,0603,UNI-ROYAL,1,0,100mW Thick Film Resistors 75V ±1% ±100ppm/℃ 10kΩ 0603 Chip Resistor - Surface Mount ROHS,30000000,"[{""qFrom"": 1, ""qTo"": 99, ""price"": 0.0007}, {""qFrom"": 100, ""price"": 0.0005}]"
98220,Resistors,Chip Resistor - Surface Mount,EXT-10K,0603,Other,0,0,±1% 10kΩ 0603 Chip Resistor,500000,"[{""qFrom"": 1, ""price"": 0.0001}]"
25805,Resistors,Chip Resistor - Surface Mount,0603WAF1103T5E,0603,UNI-ROYAL,1,0,±1% 110kΩ 0603 Chip Resistor,900000,"[{""qFrom"": 1, ""price"": 0.0002}]"
99999,Resistors,Chip Resistor - Surface Mount,LOW-STOCK,0603,Other,1,0,±1% 10kΩ 0603 Chip Resistor,3,"[{""qFrom"": 1, ""price"": 0.0001}]"
25744,Resistors,Chip Resistor - Surface Mount,0402WGF1002TCE,0402,UNI-ROYAL,1,0,±1% 10kΩ 0402 Chip Resistor,900000,"[{""qFrom"": 1, ""price"": 0.0001}]"
25806,Resistors,Chip Resistor - Surface Mount,PRECISE-10K,0603,Other,1,0,±0.1% 10kΩ 0603 Chip Resistor,900000,"[{""qFrom"": 1, ""price"": 0.0001}]"
23162,Resistors,Chip Resistor - Surface Mount,0603WAF4701T5E,0603,UNI-ROYAL,1,0,±1% 4.7kΩ 0603 Chip Resistor,900000,"[{""qFrom"": 1, ""price"": 0.0006}]"
"#;

fn resolve_parts(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new("python3")
        .arg("hardware/parts.py")
        .arg("resolve")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap()
}

#[test]
fn a_picked_part_resolves_to_a_locked_lcsc_number_that_reaches_the_bom() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let cat = tmp.path().join("catalogue.csv");
    std::fs::write(&cat, CATALOGUE).unwrap();
    let cat = cat.to_str().unwrap();
    add_board_part(
        &root,
        "id = \"pull\"\nname = \"Pull-up\"\nbody_mm = [1.6, 0.8, 0.5]\npick = { category = \"Resistors\", package = \"0603\", value = \"10k\", has = [\"1%\"] }",
    );

    // Unanswered, derive refuses: a BOM line without a part number is not orderable.
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("parts.py resolve"), "{}", text(&out));

    // Basic before extended, the value and the tolerance matched as tokens
    // (not 110k, not ±0.1 %), the package exact, enough in stock: one answer.
    let out = resolve_parts(&root, &["--catalogue", cat]);
    assert!(out.status.success(), "{}", text(&out));
    let lock = std::fs::read_to_string(root.join("hardware/parts.lock")).unwrap();
    assert!(lock.contains("lcsc = \"C25804\""), "{lock}");
    assert!(lock.contains("mpn = \"0603WAF1002T5E\""), "{lock}");

    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let bom = std::fs::read_to_string(root.join("hardware/generated/bom.csv")).unwrap();
    let line = bom.lines().find(|l| l.starts_with("pull,")).unwrap();
    assert!(
        line.contains("C25804") && line.contains("0603WAF1002T5E"),
        "{line}"
    );

    // Locked: a cheaper basic part appearing later does not move it.
    std::fs::write(
        tmp.path().join("catalogue.csv"),
        format!("{CATALOGUE}11111,Resistors,Chip,CHEAPER,0603,X,1,0,±1% 10kΩ 0603,9000000,\"[{{\"\"qFrom\"\": 1, \"\"price\"\": 0.00001}}]\"\n"),
    )
    .unwrap();
    let out = resolve_parts(&root, &["--catalogue", cat]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        std::fs::read_to_string(root.join("hardware/parts.lock")).unwrap(),
        lock
    );

    // A changed pick is an unanswered one, in derive and in --check.
    edit(
        &root,
        "hardware/product.toml",
        "value = \"10k\"",
        "value = \"4k7\"",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("`pull`"), "{}", text(&out));
    let out = resolve_parts(&root, &["--catalogue", cat]);
    assert!(out.status.success(), "{}", text(&out));
    let lock = std::fs::read_to_string(root.join("hardware/parts.lock")).unwrap();
    assert!(lock.contains("lcsc = \"C23162\""), "{lock}");

    // Nothing matches: named, and the lock is not invented.
    edit(
        &root,
        "hardware/product.toml",
        "value = \"4k7\"",
        "value = \"33k\"",
    );
    let out = resolve_parts(&root, &["--catalogue", cat]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("pull: nothing in stock"),
        "{}",
        text(&out)
    );
}

#[test]
fn a_part_both_pinned_and_picked_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    add_board_part(
        &root,
        "id = \"r\"\nname = \"R\"\nbody_mm = [1.6, 0.8, 0.5]\nlcsc = \"C25804\"\npick = { package = \"0603\", value = \"10k\" }",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("pinned or picked"), "{}", text(&out));
}

#[test]
fn a_pinned_lcsc_number_is_verified_against_what_it_names() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    // LCSC's product record, served from a file: the shape its API answers in.
    let lcsc = tmp.path().join("lcsc");
    std::fs::create_dir(&lcsc).unwrap();
    std::fs::write(
        lcsc.join("C25804.json"),
        r#"{"result": {"productCode": "C25804", "productModel": "0603WAF1002T5E", "brandNameEn": "UNI-ROYAL", "encapStandard": "0603", "productDescEn": "10kΩ ±1% 100mW 0603 Thick Film Resistor"}}"#,
    )
    .unwrap();
    std::fs::write(lcsc.join("C1.json"), r#"{"result": null}"#).unwrap();
    let detail = format!("file://{}/{{code}}.json", lcsc.display());
    let resolve = |root: &Path| {
        Command::new("python3")
            .args(["hardware/parts.py", "resolve"])
            .current_dir(root)
            .env("FID_LCSC_DETAIL", &detail)
            .output()
            .unwrap()
    };
    add_board_part(
        &root,
        "id = \"r\"\nname = \"R\"\nbody_mm = [1.6, 0.8, 0.5]\nlcsc = \"C25804\"\nmpn = \"0603WAF1002T5E\"",
    );
    let out = resolve(&root);
    assert!(out.status.success(), "{}", text(&out));
    let lock = std::fs::read_to_string(root.join("hardware/parts.lock")).unwrap();
    assert!(lock.contains("verified = "), "{lock}");
    // A verified pin in the lock is not a pick: derive reads it without complaint.
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));

    // The number names another part than the one declared: refused, named.
    edit(
        &root,
        "hardware/product.toml",
        "mpn = \"0603WAF1002T5E\"",
        "mpn = \"RC0603FR-0710KL\"",
    );
    let out = resolve(&root);
    assert!(
        out.status.success(),
        "a locked pin is not re-checked: {}",
        text(&out)
    );
    let out = Command::new("python3")
        .args(["hardware/parts.py", "resolve", "--update"])
        .current_dir(&root)
        .env("FID_LCSC_DETAIL", &detail)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(
        text(&out).contains("not the declared mpn RC0603FR-0710KL"),
        "{}",
        text(&out)
    );

    // A number LCSC does not have.
    edit(
        &root,
        "hardware/product.toml",
        "lcsc = \"C25804\"",
        "lcsc = \"C1\"",
    );
    let out = resolve(&root);
    assert!(!out.status.success());
    assert!(text(&out).contains("LCSC has no part C1"), "{}", text(&out));
}

/// An MCU whose pins have I/O names, as an RP2040's do.
const MCU_SYMBOLS: &str = r#"(kicad_symbol_lib (version 20220914)
  (symbol "Device:R" (property "Reference" "R" (at 0 0 0))
    (symbol "R_0_1" (rectangle (start -1 2.5) (end 1 -2.5)))
    (symbol "R_1_1"
      (pin passive line (at 0 3.81 270) (length 1.27) (name "~" (effects)) (number "1" (effects)))
      (pin passive line (at 0 -3.81 90) (length 1.27) (name "~" (effects)) (number "2" (effects)))))
  (symbol "Lib:MCU" (property "Reference" "U" (at 0 0 0))
    (symbol "MCU_0_1" (rectangle (start -5 5) (end 5 -5)))
    (symbol "MCU_1_1"
      (pin power_in line (at -7.62 2.54 0) (length 2.54) (name "VDD" (effects)) (number "1" (effects)))
      (pin power_in line (at -7.62 0 0) (length 2.54) (name "GND" (effects)) (number "2" (effects)))
      (pin bidirectional line (at 7.62 2.54 180) (length 2.54) (name "GPIO4" (effects)) (number "3" (effects)))
      (pin bidirectional line (at 7.62 0 180) (length 2.54) (name "GPIO5" (effects)) (number "4" (effects))))))
"#;

/// Compile a firmware stub against the derived `board.rs`, as the firmware
/// would: the pin comes from the peripherals by the macro's name.
fn firmware_compiles(root: &Path, tmp: &Path, net_macro: &str) -> std::process::Output {
    let src = tmp.join("firmware.rs");
    std::fs::write(
        &src,
        format!(
            "#[path = \"{}\"]\nmod board;\n#[allow(non_snake_case)]\npub struct Peripherals {{ pub PIN_4: u8, pub PIN_5: u8 }}\n\
             pub fn pin(p: Peripherals) -> u8 {{ board::{net_macro}!(p) }}\n",
            root.join("hardware/generated/board.rs").display()
        ),
    )
    .unwrap();
    Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into()))
        .args(["--edition", "2021", "--crate-type", "lib", "--out-dir"])
        .arg(tmp)
        .arg(&src)
        .output()
        .unwrap()
}

#[test]
fn a_moved_pin_reaches_the_firmware_and_a_renamed_net_breaks_code_still_using_it() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    std::fs::create_dir_all(root.join("hardware/lib")).unwrap();
    std::fs::write(root.join("hardware/lib/symbols.kicad_sym"), MCU_SYMBOLS).unwrap();
    edit(
        &root,
        "hardware/product.toml",
        "body_mm = [18.0, 18.0, 3.0]",
        "body_mm = [18.0, 18.0, 3.0]\nsymbol  = \"Lib:MCU\"\npins    = { VDD = \"VCC\", GND = \"GND\", GPIO4 = \"SENSOR_SDA\", GPIO5 = \"LED\" }",
    );
    let p = root.join("hardware/product.toml");
    let s = std::fs::read_to_string(&p).unwrap();
    std::fs::write(
        &p,
        s + "\n[[part]]\nid = \"r-sda\"\nname = \"SDA pull-up\"\nplace = \"board\"\nmount = \"smd\"\nbody_mm = [1.6, 0.8, 0.45]\nsymbol = \"Device:R\"\npins = { 1 = \"SENSOR_SDA\", 2 = \"VCC\" }\n\
             \n[[part]]\nid = \"r-led\"\nname = \"LED resistor\"\nplace = \"board\"\nmount = \"smd\"\nbody_mm = [1.6, 0.8, 0.45]\nsymbol = \"Device:R\"\npins = { 1 = \"LED\", 2 = \"GND\" }\n",
    )
    .unwrap();

    // No [firmware] yet: the interface is derived, the pin map is not asked for.
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(!root.join("hardware/generated/board.rs").exists());
    let ts = std::fs::read_to_string(root.join("hardware/generated/interface.ts")).unwrap();
    assert!(
        ts.contains("\"SENSOR_SDA\"") && ts.contains("export type Net"),
        "{ts}"
    );

    let s = std::fs::read_to_string(&p).unwrap();
    std::fs::write(&p, s + "\n[firmware]\nmcu = \"mcu\"\n").unwrap();
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let iface: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("hardware/generated/interface.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(iface["firmware"]["pins"]["SENSOR_SDA"], "GPIO4");
    assert!(iface["nets"]["SENSOR_SDA"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("U1.GPIO4")));
    let rs = std::fs::read_to_string(root.join("hardware/generated/board.rs")).unwrap();
    assert!(
        rs.contains("macro_rules! sensor_sda") && rs.contains("$p.PIN_4"),
        "{rs}"
    );
    let out = firmware_compiles(&root, tmp.path(), "sensor_sda");
    assert!(out.status.success(), "{}", text(&out));

    // Firmware that names a pin itself would compile through a move and miss
    // it, so both derive and --check refuse it — naming file and line —
    // unless the line says it is deliberate.
    std::fs::create_dir_all(root.join("firmware/src")).unwrap();
    let main = root.join("firmware/src/main.rs");
    std::fs::write(&main, "fn f(p: P) {\n    let sda = p.PIN_4;\n}\n").unwrap();
    for args in [
        &["derive", "--check", "--pipeline", "hardware"][..],
        &["derive", "--pipeline", "hardware"],
    ] {
        let out = fid(&root, args);
        assert!(!out.status.success(), "a literal pin passed {args:?}");
        assert!(
            text(&out).contains("firmware/src/main.rs:2: takes PIN_4 directly"),
            "{}",
            text(&out)
        );
    }
    std::fs::write(&main, "fn f(p: P) {\n    let sda = p.PIN_4; // fid: allow-pin\n    // p.PIN_9 in a comment is not code\n}\n").unwrap();
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    // An I²C address typed into a transfer, once a part declares its own.
    let toml = std::fs::read_to_string(&p).unwrap();
    std::fs::write(
        &p,
        toml.replacen("id = \"r-sda\"", "i2c_address = 0x38\nid = \"r-sda\"", 1),
    )
    .unwrap();
    std::fs::write(
        &main,
        "fn f(i2c: I) {\n    i2c.write_async(0x39, &[0]);\n}\n",
    )
    .unwrap();
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success(), "a typed address passed");
    assert!(
        text(&out).contains("firmware/src/main.rs:2: addresses an I²C device as 0x39 directly"),
        "{}",
        text(&out)
    );
    std::fs::write(&p, toml).unwrap();
    std::fs::remove_dir_all(root.join("firmware")).unwrap();

    // Swap the two pins in the declaration: the firmware follows, unedited.
    edit(
        &root,
        "hardware/product.toml",
        "GPIO4 = \"SENSOR_SDA\", GPIO5 = \"LED\"",
        "GPIO4 = \"LED\", GPIO5 = \"SENSOR_SDA\"",
    );
    let out = fid(&root, &["derive", "--check", "--pipeline", "hardware"]);
    assert!(!out.status.success(), "a moved pin is drift until derived");
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let rs = std::fs::read_to_string(root.join("hardware/generated/board.rs")).unwrap();
    assert!(rs.contains("/// `SENSOR_SDA` — GPIO5.\npub const SENSOR_SDA: &str = \"GPIO5\";\nmacro_rules! sensor_sda {\n    ($p:expr) => {\n        $p.PIN_5"), "{rs}");
    assert!(firmware_compiles(&root, tmp.path(), "sensor_sda")
        .status
        .success());

    // Rename the net: board, schematic and firmware change together, and
    // firmware still using the old name no longer compiles.
    let s = std::fs::read_to_string(&p)
        .unwrap()
        .replace("SENSOR_SDA", "I2C_SDA");
    std::fs::write(&p, s).unwrap();
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let sch = std::fs::read_to_string(root.join("hardware/generated/board.kicad_sch")).unwrap();
    assert!(sch.contains("\"I2C_SDA\"") && !sch.contains("SENSOR_SDA"));
    let ts = std::fs::read_to_string(root.join("hardware/generated/interface.ts")).unwrap();
    assert!(
        ts.contains("\"I2C_SDA\"") && !ts.contains("SENSOR_SDA"),
        "{ts}"
    );
    let out = firmware_compiles(&root, tmp.path(), "sensor_sda");
    assert!(!out.status.success());
    assert!(text(&out).contains("sensor_sda"), "{}", text(&out));
    assert!(firmware_compiles(&root, tmp.path(), "i2c_sda")
        .status
        .success());
}

#[test]
fn a_board_too_wide_for_its_ceiling_holds_that_width_and_grows_long() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    edit(
        &root,
        "hardware/product.toml",
        "max_mm = [100.0, 100.0]",
        "max_mm = [34.0, 100.0]",
    );
    for i in 0..6 {
        add_board_part(
            &root,
            &format!("id = \"p{i}\"\nname = \"P\"\nbody_mm = [6.0, 6.0, 1.0]"),
        );
    }
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    let size = &l["board"]["body"]["size_mm"];
    assert_eq!(size[0].as_f64().unwrap(), 34.0, "{size}");
    assert!(size[1].as_f64().unwrap() > 34.0, "{size}");
    assert!(
        l["why"].to_string().contains("held at its ceiling"),
        "{}",
        l["why"]
    );
}

#[test]
fn a_press_fit_lid_over_a_snap_board_raises_the_base_so_its_skirt_clears() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    // A seal of kind "none" needs no gasket dimensions.
    let p = root.join("hardware/product.toml");
    let s = std::fs::read_to_string(&p).unwrap();
    let start = s.find("[case.seal]").unwrap();
    let end = start + s[start..].find("\n\n").unwrap();
    std::fs::write(
        &p,
        format!("{}[case.seal]\nkind = \"none\"{}", &s[..start], &s[end..]),
    )
    .unwrap();
    edit(
        &root,
        "hardware/product.toml",
        "max_spacing_mm = 80.0",
        "max_spacing_mm = 80.0\nclosure = \"press-fit\"",
    );
    edit(
        &root,
        "hardware/product.toml",
        "max_mm = [100.0, 100.0]",
        "max_mm = [100.0, 100.0]\nmount = \"snap\"",
    );
    // A low cavity, so the skirt is what sets the base: the seed's cell is tall.
    edit(
        &root,
        "hardware/product.toml",
        "body_mm = [70.0, 21.0, 21.0]",
        "body_mm = [30.0, 8.0, 4.0]",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    let why = l["why"].to_string();
    assert!(
        why.contains("skirt clears the board and its snap hooks"),
        "{why}"
    );
    // The skirt's lower edge is above the hooks' top: board top + 1.4 mm.
    let base = l["case"]["base_height_mm"].as_f64().unwrap();
    let z_top =
        l["board"]["z_bottom_mm"].as_f64().unwrap() + l["board"]["thickness_mm"].as_f64().unwrap();
    let skirt = l["case"]["press_fit"]["skirt_mm"].as_f64().unwrap();
    assert!(
        base - skirt >= z_top + 1.4,
        "base {base}, skirt {skirt}, board top {z_top}"
    );
}

#[test]
fn a_resolved_upgrade_conflict_is_recorded_and_not_written_again() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let lock_path = root.join("fiducial.lock");
    let file = "hardware/build.sh";
    // Installed from an older template whose `set` line differs from today's,
    // and changed here on that same line: a true conflict.
    let lock = std::fs::read_to_string(&lock_path).unwrap();
    let start = lock.find("[templates.\"hardware/build.sh\"]").unwrap();
    let end = lock[start + 1..]
        .find("\n[")
        .map_or(lock.len(), |i| start + 1 + i);
    let record = lock[start..end].replacen("set -euo pipefail", "set -eu", 1);
    assert_ne!(record, lock[start..end], "the base carries the line");
    std::fs::write(
        &lock_path,
        format!("{}{}{}", &lock[..start], record, &lock[end..]),
    )
    .unwrap();
    edit(&root, file, "set -euo pipefail", "set -eux");

    let out = fid(&root, &["upgrade", "--capability", "hardware"]);
    assert!(
        text(&out).contains("hardware/build.sh: CONFLICT"),
        "{}",
        text(&out)
    );
    assert!(std::fs::read_to_string(root.join(file))
        .unwrap()
        .contains("<<<<<<<"));

    // Unresolved, a second run refuses rather than nesting markers.
    let out = fid(&root, &["upgrade", "--capability", "hardware"]);
    assert!(text(&out).contains("UNRESOLVED"), "{}", text(&out));

    // Resolved by hand: the next run accepts it, and the one after has nothing to do.
    let s = std::fs::read_to_string(root.join(file)).unwrap();
    let start = s.find("<<<<<<<").unwrap();
    let end = s.find(">>>>>>>").unwrap() + s[s.find(">>>>>>>").unwrap()..].find('\n').unwrap() + 1;
    std::fs::write(
        root.join(file),
        format!("{}set -euxo pipefail\n{}", &s[..start], &s[end..]),
    )
    .unwrap();
    let out = fid(&root, &["upgrade", "--capability", "hardware"]);
    assert!(!text(&out).contains("CONFLICT"), "{}", text(&out));
    let out = fid(&root, &["upgrade", "--capability", "hardware"]);
    assert!(
        text(&out).contains("hardware/build.sh: no upstream change"),
        "{}",
        text(&out)
    );
    assert!(std::fs::read_to_string(root.join(file))
        .unwrap()
        .contains("set -euxo pipefail"));
}

#[test]
fn a_conflict_resolved_to_the_platforms_version_is_not_drift() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let lock_path = root.join("fiducial.lock");
    let file = "hardware/build.sh";
    let lock = std::fs::read_to_string(&lock_path).unwrap();
    let start = lock.find("[templates.\"hardware/build.sh\"]").unwrap();
    let end = lock[start + 1..]
        .find("\n[")
        .map_or(lock.len(), |i| start + 1 + i);
    // Installed from an older template: its base and its hash both differ.
    let mut record = lock[start..end].replacen("set -euo pipefail", "set -eu", 1);
    let h = record.find("hash = \"").unwrap() + 8;
    record.replace_range(h..h + 64, &"0".repeat(64));
    std::fs::write(
        &lock_path,
        format!("{}{}{}", &lock[..start], record, &lock[end..]),
    )
    .unwrap();
    edit(&root, file, "set -euo pipefail", "set -eux");
    let out = fid(&root, &["upgrade", "--capability", "hardware"]);
    assert!(text(&out).contains("CONFLICT"), "{}", text(&out));

    // Take the platform's side, as a person does for a file they never meant
    // to change: the file is now exactly what the platform ships.
    let s = std::fs::read_to_string(root.join(file)).unwrap();
    let ours = s.find("<<<<<<<").unwrap();
    let theirs = s.find("=======").unwrap();
    let close = s.find(">>>>>>>").unwrap();
    let after = close + s[close..].find('\n').unwrap() + 1;
    let line_after = |i: usize| i + s[i..].find('\n').unwrap() + 1;
    std::fs::write(
        root.join(file),
        format!(
            "{}{}{}",
            &s[..ours],
            &s[line_after(theirs)..close],
            &s[after..]
        ),
    )
    .unwrap();
    let out = fid(&root, &["doctor"]);
    assert!(
        !text(&out).contains("hardware/build.sh: modified"),
        "{}",
        text(&out)
    );
}

fn enabled(root: &Path) -> String {
    let cfg = std::fs::read_to_string(root.join("fiducial.toml")).unwrap();
    let doc: toml_edit::DocumentMut = cfg.parse().unwrap();
    doc["capabilities"]["enabled"].to_string()
}

#[test]
fn an_upgrade_keeps_the_capabilities_fid_add_declared() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let out = fid(&root, &["doctor"]);
    assert!(
        !text(&out).contains("fiducial.toml: upstream template updated"),
        "{}",
        text(&out)
    );
    let out = fid(&root, &["upgrade"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        enabled(&root).contains("\"hardware\""),
        "{}",
        enabled(&root)
    );
}

#[test]
fn an_upgrade_repairs_a_merge_base_fid_add_recorded_patched() {
    // Locks written before the fix hold the patched file as the base, so the
    // template's `enabled = []` read as an upstream change and was merged over
    // the product's capabilities — "merged cleanly".
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let cfg = std::fs::read_to_string(root.join("fiducial.toml")).unwrap();
    let lock_path = root.join("fiducial.lock");
    let mut lock: toml_edit::DocumentMut = std::fs::read_to_string(&lock_path)
        .unwrap()
        .parse()
        .unwrap();
    lock["templates"]["fiducial.toml"]["base_content"] = toml_edit::value(cfg);
    std::fs::write(&lock_path, lock.to_string()).unwrap();

    let out = fid(&root, &["upgrade"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        enabled(&root).contains("\"hardware\""),
        "{}",
        enabled(&root)
    );
    let out = fid(&root, &["doctor"]);
    assert!(
        !text(&out).contains("fiducial.toml: upstream template updated"),
        "{}",
        text(&out)
    );
}

fn hand_routed(root: &Path) {
    edit(
        root,
        "hardware/product.toml",
        "[board]",
        "[board]\nrouting = \"hand\"",
    );
}

#[test]
fn a_hand_routed_board_is_asked_for_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    hand_routed(&root);
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("there is no hardware/board-routed.kicad_pcb"),
        "{}",
        text(&out)
    );
}

#[test]
fn a_hand_routed_board_passes_until_a_part_moves_in_it() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    // The person starts from the derived board, then switches to hand.
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    hand_routed(&root);
    let derived = root.join("hardware/generated/board.kicad_pcb");
    let routed = root.join("hardware/board-routed.kicad_pcb");
    std::fs::copy(&derived, &routed).unwrap();
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let out = fid(&root, &["derive", "--check"]);
    assert!(out.status.success(), "{}", text(&out));

    // Moved in KiCad rather than in product.toml.
    let board = std::fs::read_to_string(&routed).unwrap();
    let at = board.find("(fp_text reference \"U1\"").unwrap();
    let start = board[..at].rfind("(footprint").unwrap();
    let pos = start + board[start..].find("(at ").unwrap() + 4;
    let end = pos + board[pos..].find(' ').unwrap();
    let x: f64 = board[pos..end].parse().unwrap();
    std::fs::write(
        &routed,
        format!("{}{}{}", &board[..pos], x + 2.0, &board[end..]),
    )
    .unwrap();
    let out = fid(&root, &["derive", "--check"]);
    assert!(
        !out.status.success(),
        "an edited hand-routed board is a changed input"
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("U1 moved"), "{}", text(&out));
}

#[test]
fn routing_is_auto_or_hand() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    edit(
        &root,
        "hardware/product.toml",
        "[board]",
        "[board]\nrouting = \"magic\"",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("board.routing = `magic`"),
        "{}",
        text(&out)
    );
}

#[test]
fn a_symbol_that_extends_another_is_vendored_whole() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    // A library laid out as KiCad's is: a variant that `extends` its parent.
    let lib = tmp.path().join("symbols");
    std::fs::create_dir(&lib).unwrap();
    std::fs::write(
        lib.join("Reg.kicad_sym"),
        r#"(kicad_symbol_lib (version 20220914)
  (symbol "BASE" (in_bom yes) (on_board yes)
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "BASE" (at 0 0 0))
    (symbol "BASE_0_1" (rectangle (start -5 5) (end 5 -5)))
    (symbol "BASE_1_1"
      (pin power_in line (at -7.62 0 0) (length 2.54) (name "VI" (effects)) (number "3" (effects)))
      (pin power_out line (at 7.62 0 180) (length 2.54) (name "VO" (effects)) (number "2" (effects)))))
  (symbol "VARIANT-3.3" (extends "BASE")
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "VARIANT-3.3" (at 0 0 0)))
)
"#,
    )
    .unwrap();
    edit(
        &root,
        "hardware/product.toml",
        "body_mm = [18.0, 18.0, 3.0]",
        "body_mm = [18.0, 18.0, 3.0]\nsymbol  = \"Reg:VARIANT-3.3\"",
    );
    let out = Command::new("python3")
        .args(["hardware/parts.py", "sync"])
        .current_dir(&root)
        .env("KICAD7_SYMBOL_DIR", &lib)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    assert!(!text(&out).contains("extends"), "{}", text(&out));
    let syms = std::fs::read_to_string(root.join("hardware/lib/symbols.kicad_sym")).unwrap();
    assert!(!syms.contains("(extends"), "{syms}");
    assert!(syms.contains("(symbol \"Reg:VARIANT-3.3\""), "{syms}");
    assert!(
        syms.contains("(symbol \"VARIANT-3.3_1_1\"") && syms.contains("(name \"VO\""),
        "{syms}"
    );
    assert!(
        syms.contains("(property \"Value\" \"VARIANT-3.3\""),
        "{syms}"
    );
}

/// A divider on the seed product: two resistors with real symbols, and a
/// `[[check]]` on the node between them.
fn divider_product(tmp: &Path, bottom: &str) -> std::path::PathBuf {
    let root = scaffold(tmp);
    let lib = tmp.join("symbols");
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::write(
        lib.join("Device.kicad_sym"),
        r#"(kicad_symbol_lib (version 20220914)
  (symbol "R" (in_bom yes) (on_board yes)
    (property "Reference" "R" (at 0 0 0))
    (property "Value" "R" (at 0 0 0))
    (symbol "R_1_1"
      (pin passive line (at 0 3.81 270) (length 1.27) (name "~" (effects)) (number "1" (effects)))
      (pin passive line (at 0 -3.81 90) (length 1.27) (name "~" (effects)) (number "2" (effects)))))
)
"#,
    )
    .unwrap();
    let p = root.join("hardware/product.toml");
    let mut s = std::fs::read_to_string(&p).unwrap();
    s.push_str(&format!(
        "\n[[part]]\nid = \"r-top\"\nname = \"Divider top\"\nplace = \"board\"\nmount = \"smd\"\n\
         body_mm = [2.0, 1.25, 0.6]\nsymbol = \"Device:R\"\nvalue = \"10k\"\npins = {{ 1 = \"VOUT\", 2 = \"FB\" }}\n\
         \n[[part]]\nid = \"r-bot\"\nname = \"Divider bottom\"\nplace = \"board\"\nmount = \"smd\"\n\
         body_mm = [2.0, 1.25, 0.6]\nsymbol = \"Device:R\"\nvalue = \"{bottom}\"\npins = {{ 1 = \"FB\", 2 = \"GND\" }}\n\
         \n[[part]]\nid = \"r-load\"\nname = \"Load\"\nplace = \"board\"\nmount = \"smd\"\n\
         body_mm = [2.0, 1.25, 0.6]\nsymbol = \"Device:R\"\nvalue = \"1k\"\npins = {{ 1 = \"VOUT\", 2 = \"GND\" }}\n\
         \n[[check]]\nname = \"the feedback node sits at the reference\"\n\
         drive = {{ VOUT = {{ volts = 3.3 }} }}\nexpect = {{ FB = [1.0, 1.1] }}\n"
    ));
    std::fs::write(&p, s).unwrap();
    let out = Command::new("python3")
        .args(["hardware/parts.py", "sync"])
        .current_dir(&root)
        .env("KICAD7_SYMBOL_DIR", &lib)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    root
}

#[test]
fn a_set_point_is_solved_from_the_declared_resistors() {
    // 3.3 V × 4.7 / 14.7 = 1.055 V: inside 1.0–1.1.
    let tmp = tempfile::tempdir().unwrap();
    let root = divider_product(tmp.path(), "4.7k");
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let asm = std::fs::read_to_string(root.join("hardware/generated/assembly.md")).unwrap();
    assert!(asm.contains("FB is 1.055 V, inside 1–1.1 V"), "{asm}");
}

#[test]
fn a_set_point_out_of_its_window_fails_naming_it() {
    // 3.3 V × 10 / 20 = 1.65 V: the wrong resistor, caught at derive.
    let tmp = tempfile::tempdir().unwrap();
    let root = divider_product(tmp.path(), "10k");
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains(
            "check `the feedback node sits at the reference`: FB is 1.650 V, outside 1–1.1 V"
        ),
        "{}",
        text(&out)
    );
}

#[test]
fn a_pinned_parts_price_comes_from_the_lock_and_counts_toward_the_ceiling() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    edit(
        &root,
        "hardware/product.toml",
        "body_mm = [18.0, 18.0, 3.0]",
        "body_mm = [18.0, 18.0, 3.0]\nlcsc    = \"C2040\"",
    );
    // As `parts.py resolve` writes it after verifying the number at LCSC.
    let lock = |price: f64| {
        std::fs::write(
            root.join("hardware/parts.lock"),
            format!(
                "version = 1\n\n[[part]]\nid = \"mcu\"\nlcsc = \"C2040\"\nmpn = \"RP2040\"\n\
                 unit_price = {price}\ncurrency = \"EUR\"\nat_qty = 10\nbuy_qty = 10\nverified = \"2026-10-04\"\n"
            ),
        )
        .unwrap()
    };
    lock(12.5);
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(layout(&root)["cost"]["priced_total"].as_f64(), Some(12.5));
    // Re-priced over the 30 EUR ceiling: the declared ceiling holds.
    lock(31.5);
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("priced parts already total 31.5 EUR, over the 30 ceiling"),
        "{}",
        text(&out)
    );
}

#[test]
fn the_case_colour_is_declared_and_checked() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    edit(
        &root,
        "hardware/product.toml",
        "process  = \"fdm\"",
        "process  = \"fdm\"\ncolour   = \"#1d4ed8\"",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(layout(&root)["case"]["colour"], "#1d4ed8");
    edit(
        &root,
        "hardware/product.toml",
        "colour   = \"#1d4ed8\"",
        "colour   = \"blue\"",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    assert!(text(&out).contains("`#rrggbb`"), "{}", text(&out));
}

/// A socket's land pattern: courtyard 10 × 8 mm, its metal (fab) 9 mm wide
/// with its face 0.2 mm inside the courtyard's bottom, and every pad 2 mm
/// or more behind that face. KiCad's y points down.
const SOCKET_FP: &str = r#"(footprint "Sock" (version 20221018) (generator x)
  (layer F.Cu)
  (fp_rect (start -5 -4) (end 5 4) (stroke (width 0.05) (type solid)) (layer F.CrtYd))
  (fp_rect (start -4.5 -3.5) (end 4.5 3.8) (stroke (width 0.1) (type solid)) (layer F.Fab))
  (pad 1 smd rect (at -2 1) (size 1 1) (layers F.Cu F.Mask))
  (pad 2 smd rect (at 2 1) (size 1 1) (layers F.Cu F.Mask)))"#;

fn socket_product(root: &Path) {
    std::fs::create_dir_all(root.join("hardware/lib/footprints")).unwrap();
    std::fs::write(
        root.join("hardware/lib/footprints/Sock.kicad_mod"),
        SOCKET_FP,
    )
    .unwrap();
    edit(
        root,
        "hardware/product.toml",
        "wall_mm  = 5.0",
        "wall_mm  = 5.0\ngrid_mm  = 0.5",
    );
    edit(
        root,
        "hardware/product.toml",
        "max_mm = [100.0, 100.0]",
        "max_mm = [100.0, 100.0]\nzones  = { usb = \"bottom\" }\nnear   = \"bottom\"",
    );
    add_board_part(
        root,
        "id = \"usb\"\nname = \"Socket\"\nbody_mm = [10.0, 8.0, 3.2]\nfootprint = \"Lib:Sock\"\n\
         zone = \"usb\"\nfaces = \"bottom\"\nthrough_wall = true",
    );
}

#[test]
fn a_socket_through_the_wall_stands_flush_in_a_notch_its_own_size() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    socket_product(&root);
    edit(
        &root,
        "hardware/product.toml",
        "kind              = \"gasket\"",
        "kind              = \"none\"",
    );
    edit(
        &root,
        "hardware/product.toml",
        "wall_mm  = 5.0",
        "wall_mm  = 1.6",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
    let l = layout(&root);
    let op = &l["case"]["openings"][0];
    // The window is the socket's metal plus 1 mm, not its courtyard.
    assert_eq!(op["size_mm"][0].as_f64().unwrap(), 10.0, "{op}");
    // Its mouth within 1 mm of the case's face, inside the wall: a notch the
    // board drops into from above.
    let recess = op["recess_mm"].as_f64().unwrap();
    assert!(recess <= 1.0, "{op}");
    assert_eq!(op["notch"], true, "{op}");
    // The socket may hang past the board edge as far as its pads allow: 2.5
    // mm from its courtyard's face to its pads, less 0.5 mm of copper to
    // edge and a margin, down to a tenth.
    let m: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("hardware/generated/placement-model.json")).unwrap(),
    )
    .unwrap();
    let usb = m["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == "usb")
        .unwrap();
    let past =
        m["board"]["lo_mm"][1].as_f64().unwrap() - usb["region"]["lo_mm"][1].as_f64().unwrap();
    assert!((past - 1.9).abs() < 1e-6, "{}", usb["region"]);
}

#[test]
fn a_sealed_case_keeps_a_socket_behind_its_wall_rather_than_notch_the_gasket() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    socket_product(&root);
    // Its mouth may sit deep: only the gasket is in question.
    edit(
        &root,
        "hardware/product.toml",
        "through_wall = true",
        "through_wall = true\nrecess_mm = 5.0",
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("a sealed case cannot have a notch"),
        "{}",
        text(&out)
    );
}

/// `--check` regenerates the hardware outputs and compares them, so a
/// layout the lock agrees with but this fid would not write fails — the
/// shape of a platform change to what the executor writes, which passed a
/// hash-only check while the example's `layout.json` went stale.
#[test]
fn check_regenerates_hardware_outputs_rather_than_trusting_the_lock() {
    use sha2::{Digest, Sha256};
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(fid(&root, &["derive", "--pipeline", "hardware"])
        .status
        .success());
    let path = root.join("hardware/generated/layout.json");
    let old = std::fs::read_to_string(&path).unwrap();
    let new = old.replacen("\"why\"", "\"why_not\"", 1);
    assert_ne!(old, new);
    std::fs::write(&path, &new).unwrap();
    let lock = root.join("fiducial.lock");
    let (h_old, h_new) = (
        format!("{:x}", Sha256::digest(old.as_bytes())),
        format!("{:x}", Sha256::digest(new.as_bytes())),
    );
    let l = std::fs::read_to_string(&lock).unwrap();
    assert!(l.contains(&h_old));
    std::fs::write(&lock, l.replace(&h_old, &h_new)).unwrap();
    let out = fid(&root, &["derive", "--check"]);
    assert!(!out.status.success());
    assert!(
        text(&out).contains("hardware/generated/layout.json: stale"),
        "{}",
        text(&out)
    );
}

/// A conflict in `fiducial.toml` is not written as markers: every command
/// parses that file, so markers in it stop the very commands that would say
/// what to resolve. Found re-baselining a product made with 0.9.4.
#[test]
fn an_upgrade_never_leaves_conflict_markers_in_the_declaration() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let lock_path = root.join("fiducial.lock");
    let lock = std::fs::read_to_string(&lock_path).unwrap();
    let start = lock.find("[templates.\"fiducial.toml\"]").unwrap();
    // To the end of its multi-line `base_content`, which has `[` lines of its own.
    let open = start + lock[start..].find("base_content = \"\"\"").unwrap() + 18;
    let end = open + lock[open..].find("\"\"\"").unwrap();
    // The base, as an older template wrote it, on the very line `fid add`
    // has since changed here: a true conflict.
    let record = lock[start..end].replacen("enabled = []", "enabled = [1]", 1);
    assert_ne!(record, lock[start..end], "the base carries the line");
    std::fs::write(
        &lock_path,
        format!("{}{}{}", &lock[..start], record, &lock[end..]),
    )
    .unwrap();
    let before = std::fs::read_to_string(root.join("fiducial.toml")).unwrap();

    let out = fid(&root, &["upgrade"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains("fiducial.toml: yours"),
        "{}",
        text(&out)
    );
    assert_eq!(
        std::fs::read_to_string(root.join("fiducial.toml")).unwrap(),
        before
    );
    let out = fid(&root, &["derive", "--pipeline", "hardware"]);
    assert!(out.status.success(), "{}", text(&out));
}
