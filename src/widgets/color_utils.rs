use plotters::prelude::RGBAColor;

/// Lookup the Viridis color for a given value in the interval [0.0, 1.0].
/// The color is interpolated linearly between the 255 colors.
///
/// Colors where obtained via:
///
/// ```
/// import matplotlib.pyplot as plt
///
/// # Evaluate the "viridis" colormap at 255 equidistant points
/// viridis = plt.cm.get_cmap("viridis", 255)
/// colors = [f"#{int(r*255):02x}{int(g*255):02x}{int(b*255):02x}" for r, g, b, _ in viridis.colors]
///
/// # Print the colors as a list of RGB hex expressions
/// print(colors)
/// ```
///
pub fn get_viridis_color(x: f64) -> RGBAColor {
    // List of Viridis colors (255 colors as hex strings)
    const VIRIDIS: [&str; 255] = [
        "#440154", "#440255", "#440357", "#450558", "#45065a", "#45085b", "#46095c", "#460b5e",
        "#460c5f", "#460e61", "#470f62", "#471163", "#471265", "#471466", "#471567", "#471669",
        "#47186a", "#48196b", "#481a6c", "#481c6e", "#481d6f", "#481e70", "#482071", "#482172",
        "#482273", "#482374", "#472575", "#472676", "#472777", "#472878", "#472a79", "#472b7a",
        "#472c7b", "#462d7c", "#462f7c", "#46307d", "#46317e", "#45327f", "#45347f", "#453580",
        "#453681", "#443781", "#443982", "#433a83", "#433b83", "#433c84", "#423d84", "#423e85",
        "#424085", "#414186", "#414286", "#404387", "#404487", "#3f4587", "#3f4788", "#3e4888",
        "#3e4989", "#3d4a89", "#3d4b89", "#3d4c89", "#3c4d8a", "#3c4e8a", "#3b508a", "#3b518a",
        "#3a528b", "#3a538b", "#39548b", "#39558b", "#38568b", "#38578c", "#37588c", "#37598c",
        "#365a8c", "#365b8c", "#355c8c", "#355d8c", "#345e8d", "#345f8d", "#33608d", "#33618d",
        "#32628d", "#32638d", "#31648d", "#31658d", "#31668d", "#30678d", "#30688d", "#2f698d",
        "#2f6a8d", "#2e6b8e", "#2e6c8e", "#2e6d8e", "#2d6e8e", "#2d6f8e", "#2c708e", "#2c718e",
        "#2c728e", "#2b738e", "#2b748e", "#2a758e", "#2a768e", "#2a778e", "#29788e", "#29798e",
        "#287a8e", "#287a8e", "#287b8e", "#277c8e", "#277d8e", "#277e8e", "#267f8e", "#26808e",
        "#26818e", "#25828e", "#25838d", "#24848d", "#24858d", "#24868d", "#23878d", "#23888d",
        "#23898d", "#22898d", "#228a8d", "#228b8d", "#218c8d", "#218d8c", "#218e8c", "#20908c",
        "#20918c", "#1f928c", "#1f938b", "#1f948b", "#1f958b", "#1f968b", "#1e978a", "#1e988a",
        "#1e998a", "#1e998a", "#1e9a89", "#1e9b89", "#1e9c89", "#1e9d88", "#1e9e88", "#1e9f88",
        "#1ea087", "#1fa187", "#1fa286", "#1fa386", "#20a485", "#20a585", "#21a685", "#21a784",
        "#22a784", "#23a883", "#23a982", "#24aa82", "#25ab81", "#26ac81", "#27ad80", "#28ae7f",
        "#29af7f", "#2ab07e", "#2bb17d", "#2cb17d", "#2eb27c", "#2fb37b", "#30b47a", "#32b57a",
        "#33b679", "#35b778", "#36b877", "#38b976", "#39b976", "#3bba75", "#3dbb74", "#3ebc73",
        "#40bd72", "#42be71", "#44be70", "#45bf6f", "#47c06e", "#49c16d", "#4bc26c", "#4dc26b",
        "#4fc369", "#51c468", "#53c567", "#55c666", "#57c665", "#59c764", "#5bc862", "#5ec961",
        "#60c960", "#62ca5f", "#64cb5d", "#67cc5c", "#69cc5b", "#6bcd59", "#6dce58", "#70ce56",
        "#72cf55", "#74d054", "#77d052", "#79d151", "#7cd24f", "#7ed24e", "#81d34c", "#83d34b",
        "#86d449", "#88d547", "#8bd546", "#8dd644", "#90d643", "#92d741", "#95d73f", "#97d83e",
        "#9ad83c", "#9dd93a", "#9fd938", "#a2da37", "#a5da35", "#a7db33", "#aadb32", "#addc30",
        "#afdc2e", "#b2dd2c", "#b5dd2b", "#b7dd29", "#bade27", "#bdde26", "#bfdf24", "#c2df22",
        "#c5df21", "#c7e01f", "#cae01e", "#cde01d", "#cfe11c", "#d2e11b", "#d4e11a", "#d7e219",
        "#dae218", "#dce218", "#dfe318", "#e1e318", "#e4e318", "#e7e419", "#e9e419", "#ece41a",
        "#eee51b", "#f1e51c", "#f3e51e", "#f6e61f", "#f8e621", "#fae622", "#fde724",
    ];

    // Clamp x to the range [0.0, 1.0]
    let x = x.clamp(0.0, 1.0);

    // Scale x to the range [0, 254]
    let scaled_x = x * 254.0;
    let idx = scaled_x.floor() as usize;
    let t = scaled_x - idx as f64;

    // Get the two colors for interpolation
    let color1 = hex_to_rgb(VIRIDIS[idx]);
    let color2 = hex_to_rgb(VIRIDIS[(idx + 1).min(254)]);

    // Interpolate the colors
    let r = (1.0 - t) * color1.0 as f64 + t * color2.0 as f64;
    let g = (1.0 - t) * color1.1 as f64 + t * color2.1 as f64;
    let b = (1.0 - t) * color1.2 as f64 + t * color2.2 as f64;

    RGBAColor(r.round() as u8, g.round() as u8, b.round() as u8, 1.0)
}

/// Convert a hex color string (e.g., "#112233") to an (r, g, b) tuple.
fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let r = u8::from_str_radix(&hex[1..3], 16).unwrap();
    let g = u8::from_str_radix(&hex[3..5], 16).unwrap();
    let b = u8::from_str_radix(&hex[5..7], 16).unwrap();
    (r, g, b)
}
