#import "@preview/cetz:0.4.2"

#let vc_add = (coordinate, delta) => (coordinate.at(0) + delta.at(0), coordinate.at(1) + delta.at(1))
#let vc_sub = (coordinate, delta) => (coordinate.at(0) - delta.at(0), coordinate.at(1) - delta.at(1))
#let vc_scale = (coordinate, factor) => (coordinate.at(0) * factor, coordinate.at(1) * factor)
#let vc_midpoint = (coord_a, coord_b, pos: 50%) => {
  let t = pos / 100%
  (
    coord_a.at(0) + t * (coord_b.at(0) - coord_a.at(0)),
    coord_a.at(1) + t * (coord_b.at(1) - coord_a.at(1)),
  )
}
#let vc_distance = (coord_a, coord_b) => {
  let dx = coord_b.at(0) - coord_a.at(0)
  let dy = coord_b.at(1) - coord_a.at(1)
  calc.sqrt(dx * dx + dy * dy)
}

#let arrow = (
  positions,
  stroke: 1pt + black,
  side: 3,
  radius: 0.2,
  symbol: ">",
  mark_fill: black,
  mark_scale: 1,
) => {
  let _stroke = stroke

  import cetz.draw: *
  let previous_true_end = none
  let previous_end = none

  for (idx, (a, b)) in positions.zip(positions.slice(1)).enumerate() {
    let has_next = idx < positions.len() - 2
    let has_previous = idx > 0

    let a_dir = vc_scale(vc_sub(b, a), 1 / (vc_distance(a, b) + 1e-6))

    // Compute the adjusted start and end points to create a gap for the arrowhead, while ensuring a minimum radius to avoid degenerate cases.
    let a_prime = if has_previous {
      vc_add(a, vc_scale(a_dir, radius))
    } else { a }

    let b_prime = if has_next {
      vc_sub(b, vc_scale(a_dir, radius))
    } else {
      vc_sub(b, vc_scale(a_dir, 0.1 * mark_scale))
    }

    // Draw the line segment
    line(a_prime, b_prime, stroke: _stroke)

    // Draw the radius circles at the start and end of the segment to create rounded corners
    if has_previous {
      bezier(
        previous_end,
        a_prime,
        previous_true_end,
        stroke: _stroke,
      )
    }

    // Draw the arrowhead at the end of the last segment
    if not has_next {
      mark(
        b,
        vc_add(b, a_dir),
        scale: mark_scale,
        symbol: symbol,
        fill: mark_fill,
      )
    }


    previous_true_end = b
    previous_end = b_prime
  }
}

// #let arrow = (from, to, color: black, width: 0.5mm, side: 3, head-size: 0.1, padding: 1, horizontal: false) => {
//   import cetz.draw: *
//   let from = vc_scale(from, 1 + padding)
//   let to = vc_scale(to, 1 + padding)

//   let mid_a = (0, 0)
//   let mid_b = (0, 0)
//   if (horizontal) {
//     mid_a = (to.at(0), from.at(1))
//     mid_b = (from.at(0), to.at(1))
//   } else {
//     mid_a = (from.at(0), to.at(1))
//     mid_b = (to.at(0), from.at(1))
//   }

//   let direction = vc_scale(vc_sub(to, mid_b), 1 / (vc_distance(mid_b, to) + 1e-3))
//   let angle = calc.atan2(direction.at(0), direction.at(1))
//   let to = vc_sub(to, vc_scale(direction, head-size))

//   // Project mid on either the
//   // line(from, mid_a, mid_b, to, stroke: blue + width)
//   bezier(
//     from,
//     to,
//     mid_a,
//     mid_b,
//     stroke: color + width,
//   )
//   polygon(
//     to,
//     side,
//     angle: angle + 0deg,
//     radius: head-size,
//     fill: color,
//   )
// }

#let blob = (_content, coord_a, size: (3, 1), color: red, text_padding: 0.3, text_anchor: "center") => {
  import cetz.draw: *
  let coord_b = vc_add(coord_a, size)
  let coord_c = vc_midpoint(coord_a, coord_b)

  if text_anchor.starts-with("top-") {
    coord_c = (coord_c.at(0), coord_b.at(1) - text_padding)
  } else if text_anchor.starts-with("bottom-") {
    coord_c = (coord_c.at(0), coord_a.at(1) + text_padding)
  }
  if text_anchor.ends-with("-left") {
    coord_c = (coord_a.at(0) + text_padding, coord_c.at(1))
  } else if text_anchor.ends-with("-right") {
    coord_c = (coord_b.at(0) - text_padding, coord_c.at(1))
  }

  rect(
    coord_a,
    coord_b,
    fill: color.lighten(60%),
    stroke: color.darken(30%) + 0.5mm,
    anchor: "center",
    radius: 2mm,
  )
  content(coord_c, text(fill: color.darken(60%), size: 1.1em, font: "Open Sans", _content))
}
