//! Setter's design system: typed, reusable components on top of the official
//! Dioxus components (`crate::components`), styled only through tokens.css.
//! Pages compose these; they never style raw elements themselves.

pub mod answer_bar;
pub mod answer_nav;
pub mod chip;
pub mod clue_field;
pub mod clue_sheet;
pub mod crossword_grid;
mod discipline;
pub mod empty_state;
pub mod icon;
pub mod inline_edit;
pub mod menu_button;
pub mod mini_grid;
pub mod note;
pub mod notice;
pub mod prose;
pub mod puzzle_list;
pub mod reads_as;
pub mod segmented;
pub mod side_nav;
pub mod status_line;
pub mod template_picker;
pub mod tiles;
pub mod top_bar;
pub mod word_column;
pub mod word_popover;

/// Render a component tree to HTML inside a real VirtualDom (handlers and
/// hooks need a runtime), for assertions in tests.
#[cfg(test)]
pub fn test_render(make: impl FnOnce() -> dioxus::prelude::Element + 'static) -> String {
    use dioxus::prelude::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    type Make = Rc<RefCell<Option<Box<dyn FnOnce() -> Element>>>>;
    #[derive(Clone)]
    struct Root(Make);
    impl PartialEq for Root {
        fn eq(&self, _: &Self) -> bool {
            false
        }
    }
    fn root(p: Root) -> Element {
        let make = p.0.borrow_mut().take();
        make.map(|f| f()).unwrap_or_else(VNode::empty)
    }
    let mut dom =
        VirtualDom::new_with_props(root, Root(Rc::new(RefCell::new(Some(Box::new(make))))));
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}
