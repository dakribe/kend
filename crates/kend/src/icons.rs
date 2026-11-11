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

// Message actions, in the same outline style.

pub static ARCHIVE: Icon = Icon {
    name: "kend-archive",
    view: 16.0,
    body: concat!(stroke!("M2.5 3.5H13.5V6H2.5Z"), stroke!("M3.5 6V12.5H12.5V6"), stroke!("M6.5 8.5H9.5")),
};

pub static REPLY: Icon = Icon {
    name: "kend-reply",
    view: 16.0,
    body: concat!(stroke!("M6.5 4L2.5 8L6.5 12"), stroke!("M2.5 8H9.5C11.5 8 13.5 9.5 13.5 12.5")),
};

pub static FORWARD: Icon = Icon {
    name: "kend-forward",
    view: 16.0,
    body: concat!(stroke!("M9.5 4L13.5 8L9.5 12"), stroke!("M13.5 8H6.5C4.5 8 2.5 9.5 2.5 12.5")),
};

/// A closed envelope: mark unread.
pub static UNREAD: Icon = Icon {
    name: "kend-unread",
    view: 16.0,
    body: concat!(stroke!("M2.5 4H13.5V12H2.5Z"), stroke!("M2.5 4.5L8 8.5L13.5 4.5")),
};

/// An open envelope: mark read.
pub static READ: Icon = Icon {
    name: "kend-read",
    view: 16.0,
    body: concat!(stroke!("M2.5 7L8 3L13.5 7V13H2.5Z"), stroke!("M2.5 7.5L8 10.5L13.5 7.5")),
};

/// A box with an arrow leaving it: open in the browser.
pub static EXTERNAL: Icon = Icon {
    name: "kend-external",
    view: 16.0,
    body: concat!(stroke!("M7 3.5H3.5V12.5H12.5V9"), stroke!("M9.5 3.5H12.5V6.5"), stroke!("M12.5 3.5L7.5 8.5")),
};

pub static BACK: Icon = Icon { name: "kend-back", view: 16.0, body: stroke!("M10 3.5L5.5 8L10 12.5") };

/// A pen over a page: new message.
pub static COMPOSE: Icon = Icon {
    name: "kend-compose",
    view: 16.0,
    body: concat!(stroke!("M7.5 3H3.5V12.5H13V8.5"), stroke!("M11.5 2.5L13.5 4.5L8.5 9.5H6.5V7.5Z")),
};

/// An arrow curling back: undo.
pub static UNDO: Icon = Icon {
    name: "kend-undo",
    view: 16.0,
    body: concat!(
        stroke!("M5.5 3.5L2.5 6.5L5.5 9.5"),
        stroke!("M2.5 6.5H9.5C11.75 6.5 13.5 8.25 13.5 10.25C13.5 12.25 11.75 13 9.5 13H7")
    ),
};
