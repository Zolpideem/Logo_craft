use eframe::egui::{self, Align2, Color32, FontId, Painter, Pos2, Rect, Stroke, Vec2, vec2};

use crate::model::{Block, ConnectorDrawing, DiagramDrawing, Kind};

use super::{NODE_W, in_pos, node_height, theme::*};

pub(super) fn block_size(block: &Block) -> Vec2 {
    block
        .drawing
        .as_ref()
        .map_or(vec2(NODE_W, node_height(block.kind)), |drawing| {
            vec2(drawing.size[0], drawing.size[1])
        })
}

pub(super) fn symbol(painter: &Painter, block: &Block, rect: Rect, zoom: f32) {
    let pulse_relay = block.kind == Kind::ToggleRelay;
    let label = match (block.kind, block.drawing.as_ref()) {
        (Kind::ToggleRelay, _) => "RS",
        (Kind::OpenPin, Some(drawing)) => &drawing.symbol,
        (kind, _) => kind.symbol(),
    };
    let at = if pulse_relay {
        let pulse = |points: &[(f32, f32)]| {
            painter.add(egui::Shape::line(
                points
                    .iter()
                    .map(|&(x, y)| rect.min + vec2(x * rect.width(), y * rect.height()))
                    .collect(),
                Stroke::new((0.65 * zoom).max(0.55), WIRE),
            ));
        };
        pulse(&[
            (0.10, 0.31),
            (0.23, 0.31),
            (0.23, 0.11),
            (0.37, 0.11),
            (0.37, 0.31),
            (0.61, 0.31),
            (0.61, 0.11),
            (0.75, 0.11),
            (0.75, 0.31),
            (0.90, 0.31),
        ]);
        pulse(&[
            (0.10, 0.58),
            (0.23, 0.58),
            (0.23, 0.41),
            (0.61, 0.41),
            (0.61, 0.58),
            (0.90, 0.58),
        ]);
        rect.center_bottom() - vec2(0.0, rect.height() * 0.17)
    } else {
        rect.center_top() + vec2(0.0, rect.height() / 3.0)
    };
    painter.text(
        at,
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(if pulse_relay { 9.5 } else { 12.0 } * zoom),
        DARK,
    );
}

pub(super) fn input_position(block: &Block, rect: Rect, pin: usize, zoom: f32) -> Pos2 {
    block
        .drawing
        .as_ref()
        .and_then(|drawing| drawing.inputs.get(pin).copied().flatten())
        .map_or_else(
            || in_pos(rect, pin, block.kind, zoom),
            |ends| {
                let [a, b] = ends.map(|[x, y]| rect.min + vec2(x, y) * zoom);
                if a.x > b.x { a } else { b }
            },
        )
}

pub(super) fn has_input(block: &Block, pin: usize) -> bool {
    !block
        .drawing
        .as_ref()
        .is_some_and(|drawing| drawing.inputs.get(pin).is_some_and(Option::is_none))
}

pub(super) fn output_position(block: &Block, rect: Rect, zoom: f32) -> Pos2 {
    block
        .drawing
        .as_ref()
        .and_then(|drawing| drawing.output)
        .map_or_else(
            || rect.right_center(),
            |ends| {
                let [a, b] = ends.map(|[x, y]| rect.min + vec2(x, y) * zoom);
                if a.x < b.x { a } else { b }
            },
        )
}

pub(super) fn extra_pins(painter: &Painter, block: &Block, rect: Rect, zoom: f32) {
    let Some(drawing) = &block.drawing else {
        return;
    };
    let label_pins = block.kind.pins().iter().any(|label| label.len() > 1)
        || drawing.extra_inputs.iter().any(|(name, _)| name == "Par");
    if label_pins {
        for (label, ends) in block
            .kind
            .pins()
            .iter()
            .copied()
            .zip(drawing.inputs.iter().map(Option::as_ref))
            .chain(
                drawing
                    .extra_inputs
                    .iter()
                    .map(|(label, ends)| (label.as_str(), Some(ends))),
            )
        {
            if let Some(ends) = ends {
                let [x, y] = ends[0];
                painter.text(
                    rect.min + vec2(x.min(ends[1][0]) - 1.0, y) * zoom,
                    Align2::RIGHT_CENTER,
                    label,
                    FontId::proportional(6.5 * zoom),
                    WIRE,
                );
            }
        }
    }
    if !drawing.parameter_text.trim().is_empty() {
        painter.text(
            rect.left_bottom() + vec2(3.0, 5.0) * zoom,
            Align2::LEFT_TOP,
            &drawing.parameter_text,
            FontId::proportional(9.0 * zoom),
            Color32::from_rgb(45, 103, 41),
        );
    }
}

pub(super) fn background(painter: &Painter, drawing: &DiagramDrawing, origin: Pos2, zoom: f32) {
    let stroke = Stroke::new(0.9_f32, WIRE);
    for route in &drawing.loose_wires {
        if route.len() >= 2 {
            painter.add(egui::Shape::line(
                route
                    .iter()
                    .map(|&[x, y]| origin + vec2(x, y) * zoom)
                    .collect(),
                stroke,
            ));
        }
    }
    for marker in &drawing.loose_connectors {
        connector(painter, marker, origin, zoom, stroke);
    }
    for &([x, y], ref comment) in &drawing.comments {
        painter.text(
            origin + vec2(x, y) * zoom,
            Align2::LEFT_TOP,
            comment,
            FontId::proportional(10.0 * zoom),
            Color32::from_rgb(45, 103, 41),
        );
    }
}

pub(super) fn path_distance(pointer: Pos2, points: &[Pos2]) -> f32 {
    points
        .windows(2)
        .map(|segment| {
            let axis = segment[1] - segment[0];
            let closest = if axis.length_sq() > 0.0 {
                segment[0]
                    + axis * ((pointer - segment[0]).dot(axis) / axis.length_sq()).clamp(0.0, 1.0)
            } else {
                segment[0]
            };
            closest.distance(pointer)
        })
        .min_by(f32::total_cmp)
        .unwrap_or(f32::INFINITY)
}

pub(super) fn connector(
    painter: &Painter,
    marker: &ConnectorDrawing,
    origin: Pos2,
    zoom: f32,
    stroke: Stroke,
) -> Pos2 {
    let mut points: Vec<_> = marker
        .path
        .iter()
        .map(|&[x, y]| origin + vec2(x, y) * zoom)
        .collect();
    if points.len() < 3 {
        return origin;
    }
    let bounds = Rect::from_points(&points);
    points.push(points[0]);
    let mut fill = egui::Mesh::default();
    let green = Color32::from_rgb(125, 220, 147);
    fill.colored_vertex(bounds.center(), green);
    for &point in &points {
        fill.colored_vertex(point, green);
    }
    for index in 1..points.len() as u32 {
        fill.add_triangle(0, index, index + 1);
    }
    painter.add(egui::Shape::mesh(fill));
    painter.add(egui::Shape::line(points, stroke));
    painter.text(
        bounds.left_top(),
        Align2::LEFT_BOTTOM,
        &marker.label,
        FontId::proportional(9.0 * zoom),
        WIRE,
    );
    bounds.center()
}
