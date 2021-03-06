use crate::common::{draw_polyline, BLACK, RED};
use ezgui::GfxCtx;
use geom::{Distance, PolyLine, Pt2D};

#[allow(clippy::unreadable_literal)]
pub fn run(g: &mut GfxCtx, labels: &mut Vec<(Pt2D, String)>) {
    let thin = Distance::meters(1.0);
    let width = Distance::meters(50.0);

    // TODO retain this as a regression test
    let center_pts = PolyLine::new(
        vec![
            //Pt2D::new(2623.117354164207, 1156.9671270455774),
            //Pt2D::new(2623.0950086610856, 1162.8272397294127),
            Pt2D::new(2623.0956685132396, 1162.7341864981956),
            // One problem happens starting here -- some overlap
            Pt2D::new(2622.8995366939575, 1163.2433695162579),
            Pt2D::new(2620.4658232463926, 1163.9861244298272),
            Pt2D::new(2610.979416102837, 1164.2392149291984),
            //Pt2D::new(2572.5481805300115, 1164.2059309889344),
        ]
        .iter()
        .map(|pt| Pt2D::new(pt.x() - 2500.0, pt.y() - 1000.0))
        .collect(),
    );
    draw_polyline(g, &center_pts, thin, RED);
    for (idx, pt) in center_pts.points().iter().enumerate() {
        labels.push((*pt, format!("p{}", idx + 1)));
    }

    g.draw_polygon(BLACK, &center_pts.make_polygons(width));

    // TODO colored labels!
    let side1 = center_pts.shift_right(width / 2.0).unwrap();
    //draw_polyline(g, &side1, thin, BLUE);
    for (idx, pt) in side1.points().iter().enumerate() {
        labels.push((*pt, format!("L{}", idx + 1)));
    }

    let side2 = center_pts.shift_left(width / 2.0).unwrap();
    //draw_polyline(g, &side2, thin, GREEN);
    for (idx, pt) in side2.points().iter().enumerate() {
        labels.push((*pt, format!("R{}", idx + 1)));
    }
}
