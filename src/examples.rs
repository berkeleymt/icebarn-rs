//! On-page worked examples for the Aqre puzzle round.
//!
//! Each diagram is a plain `.bpz` file in `examples/`, authored in the same
//! format as the real puzzles ([`crate::puzzles`]) and parsed the same way; the
//! `SHADE` / `XMARK` instructions encode the solution overlay. Captions and
//! section copy mirror the printed BmMT 2026 Puzzle Round packet.

use std::sync::LazyLock;

use leptos::prelude::*;

use crate::{bpz::Puzzle, editor::PuzzleGrid};

/// Parse a worked-example `.bpz` file (bundled at compile time) into a [`Puzzle`].
macro_rules! example {
    ($name:literal) => {
        LazyLock::new(|| {
            include_str!(concat!("examples/", $name, ".bpz"))
                .parse()
                .unwrap_or_else(|e| panic!("failed to parse example {}: {:?}", $name, e))
        })
    };
}

static BASIC_SAMPLE: LazyLock<Puzzle> = example!("basic-sample");
static BASIC_SOLUTION: LazyLock<Puzzle> = example!("basic-solution");
static BASIC_BAD_CONNECTED: LazyLock<Puzzle> = example!("basic-bad-connected");
static BASIC_BAD_COUNT: LazyLock<Puzzle> = example!("basic-bad-count");
static BASIC_BAD_ROW: LazyLock<Puzzle> = example!("basic-bad-row");

static PAINT_SAMPLE: LazyLock<Puzzle> = example!("paint-sample");
static PAINT_SOLUTION: LazyLock<Puzzle> = example!("paint-solution");
static PAINT_BAD: LazyLock<Puzzle> = example!("paint-bad");

static SPIRAL_SAMPLE: LazyLock<Puzzle> = example!("spiral-sample");
static SPIRAL_SOLUTION: LazyLock<Puzzle> = example!("spiral-solution");
static SPIRAL_BAD: LazyLock<Puzzle> = example!("spiral-bad");

static BINARIO_SAMPLE: LazyLock<Puzzle> = example!("binario-sample");
static BINARIO_SOLUTION: LazyLock<Puzzle> = example!("binario-solution");
static BINARIO_BAD: LazyLock<Puzzle> = example!("binario-bad");

static SPIRAL_SYM_1: LazyLock<Puzzle> = example!("spiral-sym-1");
static SPIRAL_SYM_2: LazyLock<Puzzle> = example!("spiral-sym-2");
static SPIRAL_SYM_3: LazyLock<Puzzle> = example!("spiral-sym-3");
static SPIRAL_SYM_4: LazyLock<Puzzle> = example!("spiral-sym-4");

/// A single captioned example grid.
fn diagram(puzzle: &'static Puzzle, caption: &'static str) -> AnyView {
    view! {
        <figure class="flex flex-col items-center gap-2 w-40 m-0">
            <PuzzleGrid puzzle=puzzle />
            <figcaption class="text-xs text-gray-600 text-center text-balance">{caption}</figcaption>
        </figure>
    }
    .into_any()
}

/// A wrapping row of captioned grids.
fn diagram_row(items: impl IntoIterator<Item = (&'static Puzzle, &'static str)>) -> impl IntoView {
    let views = items
        .into_iter()
        .map(|(puzzle, caption)| diagram(puzzle, caption))
        .collect::<Vec<_>>();
    view! { <div class="flex flex-wrap gap-6 items-start">{views}</div> }
}

/// A variant section: an optional heading, intro text, the sample puzzle next to
/// its correct solution(s), and (optionally) a row of incorrect solutions.
#[component]
fn ExampleSection(
    #[prop(optional)] title: &'static str,
    intro: &'static str,
    sample: &'static Puzzle,
    correct: Vec<(&'static Puzzle, &'static str)>,
    #[prop(optional)] incorrect: Vec<(&'static Puzzle, &'static str)>,
) -> impl IntoView {
    let has_incorrect = !incorrect.is_empty();
    view! {
        <section class="flex flex-col gap-3">
            {(!title.is_empty()).then(|| view! { <h4 class="font-semibold">{title}</h4> })}
            <p>{intro}</p>
            {diagram_row(std::iter::once((sample, "Sample puzzle")).chain(correct))}
            {has_incorrect
                .then(|| {
                    view! {
                        <p>"Here are some incorrect solutions:"</p>
                        {diagram_row(incorrect)}
                    }
                })}
        </section>
    }
}

#[component]
pub fn Examples() -> impl IntoView {
    view! {
        <div class="border border-gray-300 rounded-lg p-4 flex flex-col gap-6">
            <div class="flex flex-col gap-2">
                <h3 class="text-lg font-semibold">"Examples"</h3>
                <p>
                    "Shaded cells are filled gray; a "
                    <span class="text-gray-500">"✕"</span>
                    " marks a cell that is definitely unshaded."
                </p>
            </div>

            <ExampleSection
                title="Basic"
                intro="To solve the puzzle, shade cells so that all the Basic rules hold. This sample puzzle has exactly one solution:"
                sample=&BASIC_SAMPLE
                correct=vec![(&*BASIC_SOLUTION, "The only correct solution.")]
                incorrect=vec![
                    (&*BASIC_BAD_CONNECTED, "Shaded cells are not orthogonally connected."),
                    (&*BASIC_BAD_COUNT, "Wrong number of shaded cells in two regions."),
                    (&*BASIC_BAD_ROW, "Four shaded cells and four unshaded cells in a row."),
                ]
            />

            <ExampleSection
                title="Paint"
                intro="In addition to the Basic rules, each outlined region must be either fully shaded or fully unshaded:"
                sample=&PAINT_SAMPLE
                correct=vec![(&*PAINT_SOLUTION, "Correct! Each region is fully shaded or unshaded.")]
                incorrect=vec![(&*PAINT_BAD, "Incorrect. One region is not fully shaded or unshaded.")]
            />

            <section class="flex flex-col gap-3">
                <h4 class="font-semibold">"Spiral"</h4>
                <p>
                    "In addition to the Basic rules, the shaded cells in each region must have 180° rotational symmetry about the region's center:"
                </p>
                {diagram_row([
                    (&*SPIRAL_SYM_1, "Correct symmetry!"),
                    (&*SPIRAL_SYM_2, "Correct symmetry!"),
                    (&*SPIRAL_SYM_3, "Incorrect symmetry! Reflectional, but not rotational, symmetry."),
                    (&*SPIRAL_SYM_4, "Incorrect symmetry! Rotational, but not about the region's center."),
                ])}
            </section>
            <ExampleSection
                intro="Putting it together, this sample Spiral puzzle has exactly one solution:"
                sample=&SPIRAL_SAMPLE
                correct=vec![
                    (
                        &*SPIRAL_SOLUTION,
                        "Correct! All regions have 180° rotational symmetry about their centers.",
                    ),
                ]
                incorrect=vec![
                    (
                        &*SPIRAL_BAD,
                        "Incorrect. One region does not have 180° rotational symmetry about its center.",
                    ),
                ]
            />

            <ExampleSection
                title="Binario"
                intro="In addition to the Basic rules, each row (but not necessarily each column) must have the same number of shaded and unshaded cells:"
                sample=&BINARIO_SAMPLE
                correct=vec![(&*BINARIO_SOLUTION, "Correct! Each row has two shaded and two unshaded cells.")]
                incorrect=vec![
                    (&*BINARIO_BAD, "Incorrect. Rows 1 and 2 don't have equal numbers of shaded and unshaded cells."),
                ]
            />
        </div>
    }
}
