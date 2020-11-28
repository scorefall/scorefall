// ScoreFall Ink - Music Composition Software
//
// Copyright (C) 2019-2020 Jeron Aldaron Lau <jeronlau@plopgrizzly.com>
// Copyright (C) 2019-2020 Doug P. Lau
//
//     This program is free software: you can redistribute it and/or modify
//     it under the terms of the GNU General Public License as published by
//     the Free Software Foundation, either version 3 of the License, or
//     (at your option) any later version.
//
//     This program is distributed in the hope that it will be useful,
//     but WITHOUT ANY WARRANTY; without even the implied warranty of
//     MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//     GNU General Public License for more details.
//
//     You should have received a copy of the GNU General Public License
//     along with this program.  If not, see <https://www.gnu.org/licenses/>.

#![recursion_limit = "128"]
#![allow(clippy::too_many_arguments)] // js! macro causing clippy to flip out
#![allow(clippy::blacklisted_name)] // bar is a useful musical term

use cala::{info, note};

macro_rules! enclose {
    ( ($( $x:ident ),*) $y:expr ) => {
        {
            $(let $x = $x.clone();)*
            $y
        }
    };
}

use stdweb::js;
use stdweb::traits::*;
use stdweb::web::{
    document,
    event::{
        ContextMenuEvent, KeyDownEvent, KeyUpEvent, MouseWheelEvent,
        ResizeEvent,
    },
    window, IEventTarget,
};

use std::cell::RefCell;
use std::panic;
use std::rc::Rc;

use scof::{Cursor, Fraction, Pitch, Steps};
use scorefall_ink::Program;
use staverator::{BarElem, Element, SfFontMetadata, Stave, STAVE_SPACE};

use std::convert::TryInto;

mod input;

use input::*;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

const ZOOM_LEVEL: f64 = 1.0;
// Stave spaces for window height.
const WINDOW_HEIGHT_SS: i32 = 64;
const SCALEDOWN: f64 = (STAVE_SPACE * WINDOW_HEIGHT_SS) as f64 / ZOOM_LEVEL;
const SVGNS: &str = "http://www.w3.org/2000/svg";

struct State {
    program: Program,
    time_old: f64,
    #[allow(unused)] // FIXME: Implement commands.
    command: String,
    input: InputState,
    svg: stdweb::web::Element,
    meta: SfFontMetadata,
    // Window width in Stave Spaces.
    width: f32,
}

impl State {
    /// Create a new state
    fn new(svg: stdweb::web::Element) -> Result<State> {
        let cursor = document().create_element_ns(SVGNS, "rect")?;
        cursor.set_attribute("x", "0")?;
        cursor.set_attribute("y", "0")?;
        cursor.set_attribute("width", "1024")?;
        cursor.set_attribute("height", "1024")?;
        cursor.set_attribute("fill", "#FF9AF0")?;
        cursor.set_attribute("id", "cursor")?;
        let (meta, defs) = staverator::modern();
        js! {
            @{&svg}.innerHTML = @{&defs};
            @{&svg}.appendChild(@{&cursor});
        }

        Ok(State {
            program: Program::new(),
            time_old: 0.0,
            command: "".to_string(),
            input: InputState::new(),
            svg,
            meta,
            width: 0.0,
        })
    }

    /// Resize the SVG
    fn resize(&mut self) -> Result<()> {
        note!("resize");
        let svg = &self.svg;
        let height: stdweb::Value = js! { return "" + @{svg}.clientHeight; };
        let width: stdweb::Value = js! { return "" + @{svg}.clientWidth; };
        let height: f32 = height.as_str().unwrap().parse().unwrap();
        let width: f32 = width.as_str().unwrap().parse().unwrap();
        let ratio: f32 = width as f32 / height as f32;
        let viewbox = js! {
            return "0 0 " + @{SCALEDOWN} * @{ratio} + " " + @{SCALEDOWN};
        };
        svg.set_attribute("viewBox", viewbox.as_str().unwrap())?;
        self.width = ratio * WINDOW_HEIGHT_SS as f32;
        Ok(())
    }

    fn process_input(&mut self, time: f64) {
        let _dt = (time - self.time_old) as f32;
        self.time_old = time;

        let left = self.input.press(Key::Left) || self.input.press(Key::H);
        let right = self.input.press(Key::Right) || self.input.press(Key::L);
        let up = self.input.press(Key::K) || self.input.press(Key::Up);
        let down = self.input.press(Key::J) || self.input.press(Key::Down);
        // For selecting / alternate commands
        let shift =
            self.input.held(Key::LeftShift) || self.input.held(Key::RightShift);
        let alt =
            self.input.held(Key::LeftAlt) || self.input.held(Key::RightAlt);
        let ctr =
            self.input.held(Key::LeftCtrl) || self.input.held(Key::RightCtrl);
        let next_chan = (!shift && self.input.press(Key::Enter))
            || self.input.press(Key::PageDown);
        let prev_chan = (shift && self.input.press(Key::Enter))
            || self.input.press(Key::CapsLock)
            || self.input.press(Key::PageUp);
        let home = self.input.press(Key::Home);
        let end = self.input.press(Key::End);

        if self.input.has_input {
            if ctr {
                if down {
                    self.program.down_half_step();
                    self.render_measures();
                }
                if up {
                    self.program.up_half_step();
                    self.render_measures();
                }
                if right {
                    // TODO: Double duration
                }
                if left {
                    // TODO: Halve duration
                }
            } else if alt {
                if down {
                    self.program.down_quarter_step();
                    self.render_measures();
                }
                if up {
                    self.program.up_quarter_step();
                    self.render_measures();
                }
                if right {
                    // TODO: Move selection to the right
                }
                if left {
                    // TODO: Move selection to the left
                }
            } else if shift {
                if down {
                    // TODO: Select down
                }
                if up {
                    // TODO: Select up
                }
                if right {
                    // TODO: Select right
                }
                if left {
                    // TODO: Select left
                }
            } else {
                if down {
                    self.program.down_step();
                    self.render_measures();
                }
                if up {
                    self.program.up_step();
                    self.render_measures();
                }
                if left {
                    self.program.left();
                    self.render_measures();
                }
                if right {
                    self.program.right();
                    self.render_measures();
                }
            }
            // Note Lengths
            match self.input.text {
                '1' => {
                    self.program.set_dur(Fraction::new(1, 64));
                    self.render_measures();
                }
                '2' => {
                    self.program.set_dur(Fraction::new(1, 32));
                    self.render_measures();
                }
                '3' => {
                    self.program.set_dur(Fraction::new(1, 16));
                    self.render_measures();
                }
                '4' => {
                    self.program.set_dur(Fraction::new(1, 8));
                    self.render_measures();
                }
                '5' => {
                    self.program.set_dur(Fraction::new(1, 4));
                    self.render_measures();
                }
                '6' => {
                    self.program.set_dur(Fraction::new(1, 2));
                    self.render_measures();
                }
                '7' => {
                    self.program.set_dur(Fraction::new(1, 1));
                    self.render_measures();
                }
                '8' => {
                    self.program.set_dur(Fraction::new(2, 1));
                    self.render_measures();
                }
                '9' => {
                    self.program.set_dur(Fraction::new(4, 1));
                    self.render_measures();
                }
                '.' => {
                    self.program.dotted();
                    self.render_measures();
                } /*else if self.input.press(Key::T)  || self.input.press(Key::Numpad0) {
                self.program.tuplet();
                self.render_measures();
                } */
                _ => {}
            }
        }

        self.input.reset();
    }

    fn run(time: f64, rc: Rc<RefCell<Self>>) {
        rc.borrow_mut().process_input(time);

        window().request_animation_frame(move |time| {
            Self::run(time, rc);
        });
    }

    /// Initialize the score SVG
    fn initialize_score(&self) -> Result<()> {
        let page = document().create_element_ns(SVGNS, "g")?;
        page.set_attribute("id", "page")?;
        let svg = &self.svg;
        js! {
            @{svg}.appendChild(@{page});
        };
        Ok(())
    }

    /// Render the score
    fn render_score(&mut self) -> Result<()> {
        self.initialize_score()?;
        self.resize()?;
        self.render_measures();
        Ok(())
    }

    /// Render the measures to the SVG
    fn render_measures(&self) {
        note!("render measures");
        let svg = &self.svg;
        js! {
            var page = @{svg}.getElementById("page");
            page.innerHTML = "";
        };

        let mut offset_x = STAVE_SPACE; // Stave Margin
        let mut measure = 0;
        'render_measures: loop {
            let width = self.render_measure(measure, offset_x);
            note!("measure: {} width {}", measure, width);
            offset_x += width;
            if offset_x >= (self.width * STAVE_SPACE as f32) as i32 {
                break 'render_measures;
            }
            measure += 1;
        }
    }

    /// Render one measure
    fn render_measure(&self, measure: u16, offset_x: i32) -> i32 {
        let offset_y = 0;
        let bar_id = &format!("m{}", measure);
        let trans = &format!("translate({} {})", offset_x, offset_y);
        let svg = &self.svg;
        let bar_g = js! {
            var page = @{svg}.getElementById("page");
            var old_g = @{svg}.getElementById(@{bar_id});
            var bar_g = document.createElementNS(@{SVGNS}, "g");
            bar_g.setAttributeNS(null, "id", @{bar_id});
            bar_g.setAttributeNS(null, "transform", @{trans});
            if (old_g !== null) {
                old_g.replaceWith(bar_g);
            } else {
                page.appendChild(bar_g);
            }
            return bar_g;
        };

        let high = "C4".parse::<Pitch>().unwrap().visual_distance();
        let low = "C4".parse::<Pitch>().unwrap().visual_distance();

        let mut curs = Cursor::new(
            0, /*mvmt*/
            measure, 0, /*i chan*/
            0, /*marking*/
        );
        // Alto clef has 0 steps offset
        let mut bar =
            BarElem::new(Stave::new(5, Steps(4), Steps(0)), high, low);
        if let Some((cx, cy, cwidth, cheight)) = bar.add_markings(
            &self.meta,
            &self.program.scof,
            &self.program.cursor,
            &mut curs,
        ) {
            let cur: stdweb::web::Element =
                document().get_element_by_id("cursor").unwrap();

            cur.set_attribute("x", &format!("{}", cx + offset_x))
                .unwrap();
            cur.set_attribute("y", &format!("{}", cy)).unwrap();
            cur.set_attribute("width", &format!("{}", cwidth)).unwrap();
            cur.set_attribute("height", &format!("{}", cheight))
                .unwrap();
        }

        for elem in bar.elements {
            if let Some(e) = create_elem(elem) {
                js! { @{&bar_g}.appendChild(@{e}); }
            }
        }

        bar.width
    }
}

/// Create DOM element from a staverator Element
fn create_elem(elem: Element) -> Option<stdweb::Value> {
    match elem {
        Element::Rect(r) => Some(js! {
            var rect = document.createElementNS(@{SVGNS}, "rect");
            rect.setAttributeNS(null, "x", @{r.x});
            rect.setAttributeNS(null, "y", @{r.y});
            rect.setAttributeNS(null, "width", @{r.width});
            rect.setAttributeNS(null, "height", @{r.height});
            var rx = @{r.rx};
            if (rx !== null) {
                rect.setAttributeNS(null, "rx", rx);
            }
            var ry = @{r.ry};
            if (ry !== null) {
                rect.setAttributeNS(null, "ry", ry);
            }
            rect.setAttributeNS(null, "fill", @{r.fill});
            return rect;
        }),
        Element::Use(u) => {
            let xlink = format!("#{:x}", u.id);
            Some(js! {
                var stamp = document.createElementNS(@{SVGNS}, "use");
                stamp.setAttributeNS(null, "x", @{u.x});
                stamp.setAttributeNS(null, "y", @{u.y});
                stamp.setAttributeNS(null, "href", @{xlink});
                return stamp;
            })
        }
        Element::Path(p) => Some(js! {
            var path = document.createElementNS(@{SVGNS}, "path");
            path.setAttributeNS(null, "d", @{p.d});
            return path;
        }),
        _ => None,
    }
}

fn panic_hook(panic_info: &std::panic::PanicInfo) {
    let msg = panic_info.to_string();

    info!("ScoreFall Ink panicked!: {:?}", msg);
    js! { console.trace() }
    std::process::exit(0);
}

fn main() {
    stdweb::initialize();
    let hook = panic::take_hook();
    panic::set_hook(Box::new(move |p| {
        hook(p);
        panic_hook(p);
    }));

    let svg = document().get_element_by_id("canvas").unwrap();
    let state = Rc::new(RefCell::new(State::new(svg).unwrap()));

    // FIXME: Use this.
    let _prompt: stdweb::web::Element =
        document().get_element_by_id("prompt").unwrap();

    window().add_event_listener(enclose!( (state) move |_: ResizeEvent| {
        state.borrow_mut().resize().unwrap();
    }));

    window().add_event_listener(
        enclose!( (/*state*/) move |event: ContextMenuEvent| {
        //        js! {
        //            alert("success!");
        //        }
                event.prevent_default();
            }),
    );

    // CTRL-W, CTRL-Q, CTRL-T, CTRL-N aren't picked up by this (Tested chromium,
    // firefox).
    window().add_event_listener(enclose!( (state) move |event: KeyDownEvent| {
        let is = &mut state.borrow_mut().input;
        let key = event.key();
        let code = event.code();

        if code != "F11" {
            is.update(key, code, event.is_composing(), true);
            event.prevent_default();
        }
    }));
    window().add_event_listener(enclose!( (state) move |event: KeyUpEvent| {
        let is = &mut state.borrow_mut().input;
        let key = event.key();
        let code = event.code();

        if code != "F11" {
            is.update(key, code, event.is_composing(), false);
            event.prevent_default();
        }
    }));

    window().add_event_listener(
        enclose!( (/*state*/) move |event: MouseWheelEvent| {
        //        js! {
        //            alert("keydown!");
        //        }
            event.prevent_default();
        }),
    );

    state.borrow_mut().render_score().unwrap();

    State::run(0.0, state);
    stdweb::event_loop();
}