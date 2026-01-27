#import "@preview/codly:1.3.0": *
#import "@preview/codly-languages:0.1.1": *
#import "@preview/cetz:0.4.2": *
#show: codly-init.with()

#let report_title = "Hyperion: Technical Report"
#let report_subtitle = "Building a Framework for High-Level Optimizations, for Massively Scalable Programs"
#let report_author = "Guillaume Boyé"
#let report_date = datetime.today()

#set page(
  margin: (top: 22mm, bottom: 22mm, left: 20mm, right: 20mm),
)

#set text(
  font: "Libertinus Serif",
  ligatures: true,
  size: 10.5pt,
)

#set par(
  justify: true,
)

#let calc-margin(margin, shape) = if margin == auto {
  2.5 / 21 * calc.min(..shape)
} else {
  margin
}

#show heading.where(level: 1): it => {
  pagebreak(weak: true)
  set text(
    size: 1.3em,
    weight: "regular",
  )
  set align(right)
  block(above: 20pt, below: 50pt, context {
    let title-content = {
      if heading.numbering != none {
        smallcaps([Chapter #counter(heading).display(heading.numbering)])
        linebreak()
      }

      text(size: 1.3em)[*#it.body*]
    }
    title-content
    place(dx: calc-margin(page.margin.right, (page.width, page.height)), horizon + right, rect(
      fill: black,
      height: measure(title-content).height,
    ))
  })
}

// #set heading(numbering: "1.1.")
#show heading.where(level: 2): it => {
  set text(size: 1.5em, weight: "regular")
  set align(left)
  block(
    above: 30pt,
    below: 15pt,
    // fill: red,
    context {
      let title-content = {
        smallcaps[*Section #counter(heading).display(heading.numbering) *]
        h(2mm)
      }

      title-content
      it.body
    },
  )
}

#set heading(numbering: "1.")

#show raw: it => text(it, font: "JetBrainsMono NF")
#show raw.where(block: false): it => highlight(
  it,
  fill: luma(247),
  radius: 0.5pt,
  extent: 1pt,
  top-edge: 1em,
)
#codly(zebra-fill: luma(248))

#set document(
  title: report_title,
  author: report_author,
  date: report_date,
)

#let title_page() = context [
  #align(center)[
    #v(20mm)
    #text(size: 26pt, weight: "bold")[#report_title]
    #v(5mm)
    #text(size: 12pt, fill: luma(80))[#report_subtitle]
    #v(12mm)

    #image(
      "data/hyperion-icon.svg",
      width: 120mm,
    )

    #v(1fr)

    #grid(
      columns: 1fr,
      row-gutter: 4mm,
      [#text(size: 11pt)[#report_author]],
      [#text(size: 10pt, fill: luma(90))[#report_date.display()]],
    )

    #v(20mm)
  ]
]

#show: document => [
  #title_page()
  #pagebreak()
  #outline(depth: 2)
  #pagebreak()
  #document
]

#include "sections/00-introduction.typ"
#pagebreak()

#include "sections/01-roadmap.typ"
#pagebreak()

#include "sections/02-codebase-overview.typ"
#pagebreak()

#include "sections/03-ir-spec.typ"
#pagebreak()

#include "sections/04-theorem-derivation.typ"
