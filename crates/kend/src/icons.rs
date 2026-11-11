//! Sidebar icons: 16-point outlines drawn in the text colour, matching rune-kit's glyphs.

use rune::Icon;

/// One stroked path in the shared outline style.
macro_rules! stroke {
    ($d:literal) => {
        concat!(
            "<path d=\"",
            $d,
            "\" stroke=\"currentColor\" stroke-width=\"1.2\" fill=\"none\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>"
        )
    };
}

pub static INBOX: Icon = Icon {
    name: "kend-inbox",
    view: 16.0,
    body: concat!(stroke!("M4 3.5H12L13.5 9.5V12.5H2.5V9.5Z"), stroke!("M2.5 9.5H5.5L6.5 11H9.5L10.5 9.5H13.5")),
};

pub static IMPORTANT: Icon =
    Icon { name: "kend-important", view: 16.0, body: stroke!("M2.5 4H10.5L13.5 8L10.5 12H2.5L5 8Z") };

pub static SENT: Icon = Icon {
    name: "kend-sent",
    view: 16.0,
    body: concat!(stroke!("M13.5 2.5L2.5 6.5L7 9L9.5 13.5Z"), stroke!("M13.5 2.5L7 9")),
};

pub static DRAFT: Icon = Icon {
    name: "kend-draft",
    view: 16.0,
    body: concat!(stroke!("M10.5 3L13 5.5L6 12.5H3.5V10Z"), stroke!("M9 4.5L11.5 7")),
};

pub static SPAM: Icon = Icon {
    name: "kend-spam",
    view: 16.0,
    body: concat!(
        stroke!("M5.5 2.5H10.5L13.5 5.5V10.5L10.5 13.5H5.5L2.5 10.5V5.5Z"),
        stroke!("M8 5.25V8.5"),
        "<circle cx=\"8\" cy=\"10.75\" r=\"0.8\" fill=\"currentColor\"/>"
    ),
};

pub static TRASH: Icon = Icon {
    name: "kend-trash",
    view: 16.0,
    body: concat!(stroke!("M3 4.5H13"), stroke!("M6.5 4.5V3H9.5V4.5"), stroke!("M4.5 4.5L5 13H11L11.5 4.5")),
};

pub static LABEL: Icon = Icon {
    name: "kend-label",
    view: 16.0,
    body: concat!(
        stroke!("M2.5 2.5H8L13.5 8L8 13.5L2.5 8Z"),
        "<circle cx=\"5.5\" cy=\"5.5\" r=\"1\" fill=\"currentColor\"/>"
    ),
};
