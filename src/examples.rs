//! On-page worked examples for the Aqre puzzle round.
//!
//! Mirrors the examples in the printed BmMT 2026 Puzzle Round packet: a sample
//! puzzle, its unique solution, and captioned incorrect solutions for the Basic
//! rules and each variant.
//!
//! The puzzle data below is AUTO-GENERATED from the packet sources by
//! `decode_examples.py` — do not edit the data tables by hand.

use leptos::prelude::*;

pub type Region = &'static [&'static [u32]];
pub type Clues = &'static [&'static [i8]];
pub type Cells = &'static [(usize, usize)];

pub struct Diagram {
    pub caption: &'static str,
    pub shaded: Cells,
    pub xmark: Cells,
}

pub struct Family {
    pub cols: usize,
    pub rows: usize,
    pub region: Region,
    pub clues: Clues,
    pub diagrams: &'static [Diagram],
}

const THICK: &str = "2px solid #1f2937";
const THIN: &str = "1px solid #9ca3af";

/// A single static example grid: region outlines, clue numbers, shaded cells,
/// and "definitely unshaded" (✕) marks. Read-only — purely illustrative.
#[component]
fn ExampleGrid(
    family: &'static Family,
    #[prop(optional)] shaded: Cells,
    #[prop(optional)] xmark: Cells,
) -> impl IntoView {
    let rows = family.rows;
    let cols = family.cols;

    let reg = move |r: isize, c: isize| -> Option<u32> {
        if r < 0 || c < 0 || r as usize >= rows || c as usize >= cols {
            None
        } else {
            Some(family.region[r as usize][c as usize])
        }
    };

    let body = (0..rows)
        .map(|r| {
            let cells = (0..cols)
                .map(|c| {
                    let me = family.region[r][c];
                    let side = |boundary: bool| if boundary { THICK } else { THIN };
                    let (ri, ci) = (r as isize, c as isize);
                    let top = side(reg(ri - 1, ci) != Some(me));
                    let bottom = side(reg(ri + 1, ci) != Some(me));
                    let left = side(reg(ri, ci - 1) != Some(me));
                    let right = side(reg(ri, ci + 1) != Some(me));

                    let is_shaded = shaded.contains(&(r, c));
                    let bg = if is_shaded { "#d1d5db" } else { "#ffffff" };
                    let style = format!(
                        "width:2.25rem;height:2.25rem;border-top:{top};border-bottom:{bottom};\
                         border-left:{left};border-right:{right};background:{bg};"
                    );

                    let clue = family.clues[r][c];
                    let content = if clue >= 0 {
                        view! { <span class="text-sm font-medium">{clue.to_string()}</span> }
                            .into_any()
                    } else if !is_shaded && xmark.contains(&(r, c)) {
                        view! { <span class="text-xs text-gray-500">"✕"</span> }.into_any()
                    } else {
                        ().into_any()
                    };

                    view! { <td class="p-0 text-center align-middle" style=style>{content}</td> }
                })
                .collect::<Vec<_>>();
            view! { <tr>{cells}</tr> }
        })
        .collect::<Vec<_>>();

    view! {
        <table class="select-none" style="border-collapse:collapse;">
            <tbody>{body}</tbody>
        </table>
    }
}

/// An example grid with a caption underneath.
#[component]
fn ExampleDiagram(
    family: &'static Family,
    #[prop(optional)] shaded: Cells,
    #[prop(optional)] xmark: Cells,
    caption: &'static str,
) -> impl IntoView {
    view! {
        <figure class="flex flex-col items-center gap-2 w-40 m-0">
            <ExampleGrid family=family shaded=shaded xmark=xmark />
            <figcaption class="text-xs text-gray-600 text-center text-balance">{caption}</figcaption>
        </figure>
    }
}

fn diagram_views(family: &'static Family, range: std::ops::Range<usize>) -> Vec<AnyView> {
    family.diagrams[range]
        .iter()
        .map(|d| {
            view! {
                <ExampleDiagram
                    family=family
                    shaded=d.shaded
                    xmark=d.xmark
                    caption=d.caption
                />
            }
            .into_any()
        })
        .collect()
}

#[component]
fn ExampleSection(
    title: &'static str,
    intro: &'static str,
    family: &'static Family,
    /// How many leading diagrams are "correct" (shown next to the sample puzzle).
    correct: usize,
) -> impl IntoView {
    view! {
        <section class="flex flex-col gap-3">
            <h4 class="font-semibold">{title}</h4>
            <p>{intro}</p>
            <div class="flex flex-wrap gap-6 items-start">
                <ExampleDiagram family=family caption="Sample puzzle" />
                {diagram_views(family, 0..correct)}
            </div>
            {(family.diagrams.len() > correct)
                .then(|| {
                    view! {
                        <p>"Here are some incorrect solutions:"</p>
                        <div class="flex flex-wrap gap-6 items-start">
                            {diagram_views(family, correct..family.diagrams.len())}
                        </div>
                    }
                })}
        </section>
    }
}

#[component]
pub fn Examples() -> impl IntoView {
    let symmetry = SPIRAL_SYMMETRY
        .iter()
        .map(|f| {
            let d = &f.diagrams[0];
            view! {
                <ExampleDiagram family=f shaded=d.shaded xmark=d.xmark caption=d.caption />
            }
            .into_any()
        })
        .collect::<Vec<_>>();

    view! {
        <div class="border border-gray-300 rounded-lg p-4 flex flex-col gap-6">
            <div class="flex flex-col gap-2">
                <h3 class="text-lg font-semibold">"Examples"</h3>
                <p>
                    "These worked examples are the same ones from the printed packet. Shaded cells are filled gray; a "
                    <span class="text-gray-500">"✕"</span>
                    " marks a cell that is definitely unshaded."
                </p>
            </div>

            <ExampleSection
                title="Basic"
                intro="To solve the puzzle, shade cells so that all the Basic rules hold. This sample puzzle has exactly one solution:"
                family=&BASIC
                correct=1
            />

            <ExampleSection
                title="Paint"
                intro="In addition to the Basic rules, each outlined region must be either fully shaded or fully unshaded:"
                family=&PAINT
                correct=1
            />

            <section class="flex flex-col gap-3">
                <h4 class="font-semibold">"Spiral"</h4>
                <p>
                    "In addition to the Basic rules, the shaded cells in each region must have 180° rotational symmetry about the region's center:"
                </p>
                <div class="flex flex-wrap gap-6 items-start">{symmetry}</div>
            </section>
            <ExampleSection
                title=""
                intro="Putting it together, this sample Spiral puzzle has exactly one solution:"
                family=&SPIRAL
                correct=1
            />

            <ExampleSection
                title="Binario"
                intro="In addition to the Basic rules, each row (but not necessarily each column) must have the same number of shaded and unshaded cells:"
                family=&BINARIO
                correct=1
            />
        </div>
    }
}

// ── AUTO-GENERATED DATA (decode_examples.py) ───────────────────────────────

pub static BASIC: Family = Family {
    cols: 4,
    rows: 4,
    region: &[&[0, 0, 1, 1], &[0, 2, 2, 1], &[3, 4, 4, 5], &[3, 3, 4, 5]],
    clues: &[&[3, -1, 2, -1], &[-1, -1, -1, -1], &[1, 0, -1, -1], &[-1, -1, -1, -1]],
    diagrams: &[
        Diagram {
            caption: "The only correct solution.",
            shaded: &[(0, 0), (0, 1), (0, 2), (1, 0), (1, 2), (1, 3), (2, 0), (2, 3), (3, 3)],
            xmark: &[],
        },
        Diagram {
            caption: "Shaded cells are not orthogonally connected.",
            shaded: &[(0, 0), (0, 1), (0, 2), (1, 0), (1, 3), (2, 3), (3, 1)],
            xmark: &[],
        },
        Diagram {
            caption: "Wrong number of shaded cells in two regions.",
            shaded: &[(0, 0), (0, 1), (0, 2), (1, 0), (1, 2), (1, 3), (2, 0), (2, 1), (3, 1)],
            xmark: &[],
        },
        Diagram {
            caption: "Four shaded cells and four unshaded cells in a row.",
            shaded: &[(0, 0), (0, 1), (0, 2), (0, 3), (1, 0), (2, 0)],
            xmark: &[(3, 0), (3, 1), (3, 2), (3, 3)],
        },
    ],
};

pub static PAINT: Family = Family {
    cols: 4,
    rows: 4,
    region: &[&[0, 0, 0, 1], &[2, 0, 3, 1], &[2, 4, 4, 1], &[2, 2, 4, 5]],
    clues: &[&[-1, -1, -1, -1], &[-1, -1, -1, -1], &[-1, -1, -1, -1], &[-1, -1, -1, -1]],
    diagrams: &[
        Diagram {
            caption: "Correct! Each region is fully shaded or unshaded.",
            shaded: &[(0, 0), (0, 1), (0, 2), (1, 1), (2, 1), (2, 2), (3, 2), (3, 3)],
            xmark: &[],
        },
        Diagram {
            caption: "Incorrect. One region is not fully shaded or unshaded.",
            shaded: &[(0, 0), (0, 1), (0, 2), (1, 1), (1, 3), (2, 1), (2, 2), (2, 3), (3, 2)],
            xmark: &[(0, 3)],
        },
    ],
};

pub static SPIRAL: Family = Family {
    cols: 4,
    rows: 4,
    region: &[&[0, 0, 0, 1], &[2, 2, 2, 2], &[2, 2, 2, 2], &[3, 3, 3, 4]],
    clues: &[&[2, -1, -1, 1], &[-1, -1, -1, -1], &[-1, -1, -1, -1], &[1, -1, -1, 0]],
    diagrams: &[
        Diagram {
            caption: "Correct! All regions have 180° rotational symmetry about their centers.",
            shaded: &[(0, 0), (0, 2), (0, 3), (1, 0), (1, 1), (1, 2), (2, 1), (2, 2), (2, 3), (3, 1)],
            xmark: &[],
        },
        Diagram {
            caption: "Incorrect. One region does not have 180° rotational symmetry about its center.",
            shaded: &[(0, 0), (0, 2), (0, 3), (1, 0), (1, 1), (1, 3), (2, 1), (2, 2), (2, 3), (3, 1)],
            xmark: &[(1, 2), (2, 0)],
        },
    ],
};

pub static BINARIO: Family = Family {
    cols: 4,
    rows: 4,
    region: &[&[0, 0, 1, 2], &[0, 0, 1, 2], &[3, 3, 4, 5], &[3, 3, 4, 4]],
    clues: &[&[3, -1, 1, -1], &[-1, -1, -1, -1], &[1, -1, -1, 1], &[-1, -1, -1, -1]],
    diagrams: &[
        Diagram {
            caption: "Correct! Each row has two shaded and two unshaded cells.",
            shaded: &[(0, 0), (0, 1), (1, 1), (1, 2), (2, 2), (2, 3), (3, 1), (3, 2)],
            xmark: &[],
        },
        Diagram {
            caption: "Incorrect. Rows 1 and 2 don't have equal numbers of shaded and unshaded cells.",
            shaded: &[(0, 0), (1, 0), (1, 1), (1, 2), (2, 2), (2, 3), (3, 1), (3, 2)],
            xmark: &[],
        },
    ],
};

pub static SPIRAL_SYMMETRY: &[Family] = &[
    Family {
        cols: 4,
        rows: 2,
        region: &[&[0, 0, 0, 0], &[0, 0, 0, 0]],
        clues: &[&[-1, -1, -1, -1], &[-1, -1, -1, -1]],
        diagrams: &[Diagram { caption: "Correct symmetry!", shaded: &[(0, 0), (1, 3)], xmark: &[] }],
    },
    Family {
        cols: 4,
        rows: 2,
        region: &[&[0, 0, 0, 0], &[0, 0, 0, 0]],
        clues: &[&[-1, -1, -1, -1], &[-1, -1, -1, -1]],
        diagrams: &[Diagram {
            caption: "Correct symmetry!",
            shaded: &[(0, 1), (0, 2), (1, 1), (1, 2)],
            xmark: &[],
        }],
    },
    Family {
        cols: 4,
        rows: 2,
        region: &[&[0, 0, 0, 0], &[0, 0, 0, 0]],
        clues: &[&[-1, -1, -1, -1], &[-1, -1, -1, -1]],
        diagrams: &[Diagram {
            caption: "Incorrect symmetry! Reflectional, but not rotational, symmetry.",
            shaded: &[(0, 1), (0, 2), (1, 0), (1, 3)],
            xmark: &[],
        }],
    },
    Family {
        cols: 4,
        rows: 2,
        region: &[&[0, 0, 0, 0], &[0, 0, 0, 0]],
        clues: &[&[-1, -1, -1, -1], &[-1, -1, -1, -1]],
        diagrams: &[Diagram {
            caption: "Incorrect symmetry! Rotational, but not about the region's center.",
            shaded: &[(0, 1), (0, 2), (1, 0), (1, 1)],
            xmark: &[],
        }],
    },
];
