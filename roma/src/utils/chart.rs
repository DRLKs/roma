use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Configuración para un gráfico
#[derive(Debug, Clone)]
pub struct ChartConfig {
    pub width: u32,
    pub height: u32,
    pub margin_top: u32,
    pub margin_bottom: u32,
    pub margin_left: u32,
    pub margin_right: u32,
    pub title: String,
    pub x_label: String,
    pub y_label: String,
    pub x_min: Option<f64>,
    pub x_max: Option<f64>,
    pub y_min: Option<f64>,
    pub y_max: Option<f64>,
    pub x_padding_ratio: f64,
    pub y_padding_ratio: f64,
    pub x_clamp_non_negative: bool,
    pub y_clamp_non_negative: bool,
}

impl Default for ChartConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            margin_top: 40,
            margin_bottom: 60,
            margin_left: 70,
            margin_right: 40,
            title: "Chart".to_string(),
            x_label: "X".to_string(),
            y_label: "Y".to_string(),
            x_min: None,
            x_max: None,
            y_min: None,
            y_max: None,
            x_padding_ratio: 0.05,
            y_padding_ratio: 0.05,
            x_clamp_non_negative: false,
            y_clamp_non_negative: false,
        }
    }
}

/// Serie de datos para graficar
#[derive(Debug, Clone)]
pub struct Series {
    pub name: String,
    pub data: Vec<(f64, f64)>,
    pub color: String,
}

impl Series {
    pub fn new(name: &str, data: Vec<(f64, f64)>) -> Self {
        Self {
            name: name.to_string(),
            data,
            color: "#2563eb".to_string(), // Azul por defecto
        }
    }

    pub fn with_color(mut self, color: &str) -> Self {
        self.color = color.to_string();
        self
    }
}

/// Gráfico de líneas simple
pub struct LineChart {
    config: ChartConfig,
    series: Vec<Series>,
}

impl LineChart {
    pub fn new(config: ChartConfig) -> Self {
        Self {
            config,
            series: Vec::new(),
        }
    }

    pub fn add_series(&mut self, series: Series) {
        self.series.push(series);
    }

    /// Guarda el gráfico como archivo SVG
    pub fn save<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let svg = self.generate_svg();
        let mut file = File::create(path)?;
        file.write_all(svg.as_bytes())?;
        Ok(())
    }

    /// Genera el SVG como String
    pub fn generate_svg(&self) -> String {
        let mut svg = String::new();

        // Header SVG
        svg.push_str(&format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">
"#,
            self.config.width, self.config.height
        ));

        // Fondo blanco
        svg.push_str(&format!(
            r#"  <rect width="{}" height="{}" fill="white"/>"#,
            self.config.width, self.config.height
        ));
        svg.push('\n');

        // Área de dibujo
        let plot_width = self.config.width - self.config.margin_left - self.config.margin_right;
        let plot_height = self.config.height - self.config.margin_top - self.config.margin_bottom;

        // Calcular rangos de datos
        if let Some((min_x, max_x, min_y, max_y)) = self.calculate_ranges() {
            // Título
            svg.push_str(&format!(
                r#"  <text x="{}" y="25" text-anchor="middle" font-size="16" font-weight="bold">{}</text>"#,
                self.config.width / 2,
                self.config.title
            ));
            svg.push('\n');

            // Etiqueta eje X
            svg.push_str(&format!(
                r#"  <text x="{}" y="{}" text-anchor="middle" font-size="12">{}</text>"#,
                self.config.margin_left + plot_width / 2,
                self.config.height - 10,
                self.config.x_label
            ));
            svg.push('\n');

            // Etiqueta eje Y (rotada)
            svg.push_str(&format!(
                r#"  <text x="15" y="{}" text-anchor="middle" font-size="12" transform="rotate(-90 15 {})">{}</text>"#,
                self.config.margin_top + plot_height / 2,
                self.config.margin_top + plot_height / 2,
                self.config.y_label
            ));
            svg.push('\n');

            // Ejes
            self.draw_axes(&mut svg, plot_width, plot_height);

            // Dibujar grid
            self.draw_grid(
                &mut svg,
                plot_width,
                plot_height,
                min_x,
                max_x,
                min_y,
                max_y,
            );

            // Dibujar series
            for series in self.series.iter() {
                self.draw_series(
                    &mut svg,
                    series,
                    plot_width,
                    plot_height,
                    min_x,
                    max_x,
                    min_y,
                    max_y,
                );
            }

            // Leyenda
            if !self.series.is_empty() {
                self.draw_legend(&mut svg, plot_width);
            }
        }

        svg.push_str("</svg>\n");
        svg
    }

    fn calculate_ranges(&self) -> Option<(f64, f64, f64, f64)> {
        if self.series.is_empty() {
            return None;
        }

        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for series in self.series.iter() {
            for &(x, y) in &series.data {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }

        let (min_x, max_x) = Self::resolve_axis_range(
            min_x,
            max_x,
            self.config.x_min,
            self.config.x_max,
            self.config.x_padding_ratio,
            self.config.x_clamp_non_negative,
        );
        let (min_y, max_y) = Self::resolve_axis_range(
            min_y,
            max_y,
            self.config.y_min,
            self.config.y_max,
            self.config.y_padding_ratio,
            self.config.y_clamp_non_negative,
        );

        Some((min_x, max_x, min_y, max_y))
    }

    fn resolve_axis_range(
        data_min: f64,
        data_max: f64,
        forced_min: Option<f64>,
        forced_max: Option<f64>,
        padding_ratio: f64,
        clamp_non_negative: bool,
    ) -> (f64, f64) {
        let mut min = forced_min.unwrap_or(data_min);
        let mut max = forced_max.unwrap_or(data_max);

        if max > min {
            let span = max - min;
            if forced_min.is_none() {
                min -= span * padding_ratio;
            }
            if forced_max.is_none() {
                max += span * padding_ratio;
            }
        } else {
            // Degenerate range (single value): build a visible window.
            if min == 0.0 {
                max = 1.0;
            } else {
                let margin = min.abs() * 0.1;
                min -= margin;
                max += margin;
            }
        }

        if clamp_non_negative {
            min = min.max(0.0);
        }

        if max <= min {
            max = min + 1.0;
        }

        (min, max)
    }

    fn draw_axes(&self, svg: &mut String, plot_width: u32, plot_height: u32) {
        let x0 = self.config.margin_left;
        let y0 = self.config.margin_top;

        // Eje X
        svg.push_str(&format!(
            r#"  <line x1="{}" y1="{}" x2="{}" y2="{}" stroke="black" stroke-width="2"/>"#,
            x0,
            y0 + plot_height,
            x0 + plot_width,
            y0 + plot_height
        ));
        svg.push('\n');

        // Eje Y
        svg.push_str(&format!(
            r#"  <line x1="{}" y1="{}" x2="{}" y2="{}" stroke="black" stroke-width="2"/>"#,
            x0,
            y0,
            x0,
            y0 + plot_height
        ));
        svg.push('\n');
    }

    fn draw_grid(
        &self,
        svg: &mut String,
        plot_width: u32,
        plot_height: u32,
        min_x: f64,
        max_x: f64,
        min_y: f64,
        max_y: f64,
    ) {
        let x0 = self.config.margin_left;
        let y0 = self.config.margin_top;

        // Grid horizontal (5 líneas)
        for i in 0..=5 {
            let y = y0 + (plot_height as f64 * i as f64 / 5.0) as u32;
            let value = max_y - (max_y - min_y) * i as f64 / 5.0;

            // Línea de grid
            svg.push_str(&format!(
                "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#e5e7eb\" stroke-width=\"1\"/>\n",
                x0,
                y,
                x0 + plot_width,
                y
            ));

            // Etiqueta
            svg.push_str(&format!(
                "  <text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-size=\"10\" fill=\"#6b7280\">{:.2}</text>\n",
                x0 - 5,
                y + 4,
                value
            ));
        }

        // Grid vertical (5 líneas)
        for i in 0..=5 {
            let x = x0 + (plot_width as f64 * i as f64 / 5.0) as u32;
            let value = min_x + (max_x - min_x) * i as f64 / 5.0;

            // Línea de grid
            svg.push_str(&format!(
                "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#e5e7eb\" stroke-width=\"1\"/>\n",
                x, y0, x, y0 + plot_height
            ));

            // Etiqueta
            svg.push_str(&format!(
                "  <text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"10\" fill=\"#6b7280\">{:.1}</text>\n",
                x,
                y0 + plot_height + 15,
                value
            ));
        }
    }

    fn draw_series(
        &self,
        svg: &mut String,
        series: &Series,
        plot_width: u32,
        plot_height: u32,
        min_x: f64,
        max_x: f64,
        min_y: f64,
        max_y: f64,
    ) {
        if series.data.is_empty() {
            return;
        }

        let x0 = self.config.margin_left;
        let y0 = self.config.margin_top;

        let range_x = max_x - min_x;
        let range_y = max_y - min_y;

        // Crear path para la línea
        let mut path = String::from("  <path d=\"");

        for (i, &(x, y)) in series.data.iter().enumerate() {
            let px = x0 as f64 + ((x - min_x) / range_x) * plot_width as f64;
            let py = y0 as f64 + plot_height as f64 - ((y - min_y) / range_y) * plot_height as f64;

            if i == 0 {
                path.push_str(&format!("M {} {} ", px, py));
            } else {
                path.push_str(&format!("L {} {} ", px, py));
            }
        }

        path.push_str(&format!(
            r#"" fill="none" stroke="{}" stroke-width="2"/>"#,
            series.color
        ));
        svg.push_str(&path);
        svg.push('\n');

        // Dibujar puntos
        for &(x, y) in &series.data {
            let px = x0 as f64 + ((x - min_x) / range_x) * plot_width as f64;
            let py = y0 as f64 + plot_height as f64 - ((y - min_y) / range_y) * plot_height as f64;

            svg.push_str(&format!(
                r#"  <circle cx="{}" cy="{}" r="3" fill="{}"/>"#,
                px, py, series.color
            ));
            svg.push('\n');
        }
    }

    fn draw_legend(&self, svg: &mut String, plot_width: u32) {
        let plot_height = self.config.height - self.config.margin_top - self.config.margin_bottom;
        let x = self.config.margin_left + plot_width - 150;

        // Posicionar la leyenda en la esquina inferior derecha
        let legend_height = 20 + self.series.len() as u32 * 20;
        let y_start = self.config.margin_top + plot_height - legend_height - 10;
        let mut y = y_start + 20;

        // Fondo de la leyenda
        svg.push_str(&format!(
            "  <rect x=\"{}\" y=\"{}\" width=\"140\" height=\"{}\" fill=\"white\" stroke=\"#d1d5db\" stroke-width=\"1\"/>\n",
            x - 5,
            y_start + 5,
            legend_height
        ));

        for series in &self.series {
            // Línea de color
            svg.push_str(&format!(
                "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>\n",
                x,
                y,
                x + 30,
                y,
                series.color
            ));

            // Nombre
            svg.push_str(&format!(
                "  <text x=\"{}\" y=\"{}\" font-size=\"12\">{}</text>\n",
                x + 35,
                y + 4,
                series.name
            ));

            y += 20;
        }
    }
}

/// Constructor conveniente para gráficos simples
pub struct ChartBuilder {
    config: ChartConfig,
    series: Vec<Series>,
}

impl ChartBuilder {
    pub fn new() -> Self {
        Self {
            config: ChartConfig::default(),
            series: Vec::new(),
        }
    }

    pub fn title(mut self, title: &str) -> Self {
        self.config.title = title.to_string();
        self
    }

    pub fn x_label(mut self, label: &str) -> Self {
        self.config.x_label = label.to_string();
        self
    }

    pub fn y_label(mut self, label: &str) -> Self {
        self.config.y_label = label.to_string();
        self
    }

    pub fn x_min(mut self, value: f64) -> Self {
        self.config.x_min = Some(value);
        self
    }

    pub fn x_max(mut self, value: f64) -> Self {
        self.config.x_max = Some(value);
        self
    }

    pub fn y_min(mut self, value: f64) -> Self {
        self.config.y_min = Some(value);
        self
    }

    pub fn y_max(mut self, value: f64) -> Self {
        self.config.y_max = Some(value);
        self
    }

    pub fn x_clamp_non_negative(mut self) -> Self {
        self.config.x_clamp_non_negative = true;
        self
    }

    pub fn y_clamp_non_negative(mut self) -> Self {
        self.config.y_clamp_non_negative = true;
        self
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.config.width = width;
        self.config.height = height;
        self
    }

    pub fn add_series(mut self, series: Series) -> Self {
        self.series.push(series);
        self
    }

    pub fn build(self) -> LineChart {
        let mut chart = LineChart::new(self.config);
        for series in self.series {
            chart.add_series(series);
        }
        chart
    }
}

impl Default for ChartBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_ranges_keeps_positive_domains_non_negative() {
        let chart = ChartBuilder::new()
            .add_series(Series::new(
                "Eval",
                vec![(0.0, 10.0), (10.0, 20.0), (20.0, 30.0)],
            ))
            .x_clamp_non_negative()
            .y_clamp_non_negative()
            .build();

        let (min_x, _max_x, min_y, _max_y) = chart
            .calculate_ranges()
            .expect("Expected valid ranges for non-empty series");

        assert!(min_x >= 0.0);
        assert!(min_y >= 0.0);
    }

    #[test]
    fn calculate_ranges_handles_single_x_value_without_zero_division() {
        let chart = ChartBuilder::new()
            .add_series(Series::new(
                "FlatX",
                vec![(5.0, 1.0), (5.0, 2.0), (5.0, 3.0)],
            ))
            .build();

        let (min_x, max_x, _min_y, _max_y) = chart
            .calculate_ranges()
            .expect("Expected valid ranges for non-empty series");

        assert!(max_x > min_x);
    }

    #[test]
    fn test_simple_chart() {
        let data = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 4.0), (3.0, 9.0), (4.0, 16.0)];

        let series = Series::new("x²", data).with_color("#2563eb");

        let chart = ChartBuilder::new()
            .title("Test Chart")
            .x_label("X")
            .y_label("Y")
            .add_series(series)
            .build();

        let svg = chart.generate_svg();
        assert!(svg.contains("<?xml"));
        assert!(svg.contains("Test Chart"));
    }

    #[test]
    fn test_multiple_series() {
        let series1 =
            Series::new("Linear", vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]).with_color("#2563eb");
        let series2 = Series::new("Quadratic", vec![(0.0, 0.0), (1.0, 1.0), (2.0, 4.0)])
            .with_color("#dc2626");

        let chart = ChartBuilder::new()
            .title("Multiple Series")
            .add_series(series1)
            .add_series(series2)
            .build();

        let svg = chart.generate_svg();
        assert!(svg.contains("Linear"));
        assert!(svg.contains("Quadratic"));
    }
}
