//! The four house crests, after the home page's banners (its hogwarts.html):
//! each in a 60 x 70 box that the banner places at (16, 40), scale 1.13.

/// What a mark is painted with: the house metal, its cloth, or a dark engraving.
#[derive(Clone, Copy)]
pub enum Ink {
    Metal,
    Cloth,
    Dark,
}

/// One element of a crest: an SVG path, filled and/or stroked.
pub struct Mark {
    pub d: &'static str,
    pub fill: Option<Ink>,
    pub stroke: Option<(Ink, f32)>,
    pub dash: bool,
}

pub const GRYFFINDOR: &[Mark] = &[
    Mark {
        d: "M41 47C49 47 54 40 53 32 52.5 28 55 24 58 23 55 21 51 23 50 27 49 34 46 41 41 43Z",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "E55.0 22.0 3.6 2.4 -35.0",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M27 25C33 26 39 31 41 39 43 45 40 50 35 52 29 54 24 50 22 44 20 37 21 29 27 25Z",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M35 50C37 56 37 61 35 65L39 68 30 69C29 65 31 59 30 54Z",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M27 49C25 54 21 57 17 57L16 61 11 56C15 54 19 52 22 46Z",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M25 30C20 30 15 28 12 23L8 24 8 18C12 21 16 24 22 26Z",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M25 37C20 39 15 39 11 36L8 39 7 33C11 34 16 34 22 32Z",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M19 4L23 8 27 4 28 9 33 9 31 14 35 17 30 19 32 24 27 23 25 28 21 24 17 26 17 21 12 20 15 16 11 12 16 11 15 6Z",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M15 11C13 12 10 14 7 16 8 18 11 18 13 18 12 20 13 22 16 22 19 22 22 19 22 15 22 12 19 10 15 11Z",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "O14.6 14.2 1.0",
        fill: Some(Ink::Dark),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M7.6 16.8L12.4 18.6",
        fill: Some(Ink::Dark),
        stroke: Some((Ink::Dark, 0.8)),
        dash: false,
    },
    Mark {
        d: "M27 33C30 36 31 41 30 46",
        fill: None,
        stroke: Some((Ink::Dark, 0.7)),
        dash: false,
    },
    Mark {
        d: "M33 31C36 35 37 41 36 46",
        fill: None,
        stroke: Some((Ink::Dark, 0.7)),
        dash: false,
    },
];

pub const SLYTHERIN: &[Mark] = &[
    Mark {
        d: "M28 68C16 68 10 63 12 57 14 51 24 50 34 50 44 50 50 46 48 40 46 34 38 33 30 33 22 33 15 30 15 24 15 17 22 13 30 13 34 13 38 14 41 16L43 12C46 13 49 16 48 20 47 23 44 24 41 23 39 21 35 20 30 20 26 20 22 21 22 24 22 27 26 27 32 27 44 27 55 31 55 41 55 52 45 56 35 56 25 56 20 57 20 60 20 63 26 64 32 63L32 68Z",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M41 16C44 9 50 6 55 8 59 10 58 15 55 18 52 21 46 21 41 19Z",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M58 12L62 10M58 13.4L62 14.6",
        fill: None,
        stroke: Some((Ink::Metal, 0.9)),
        dash: false,
    },
    Mark {
        d: "O52.5 11.6 1.1",
        fill: Some(Ink::Dark),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M24 59C30 58 40 59 46 53M20 25C24 23 32 24 38 24M45 30C50 33 52 38 50 44M17 21C18 16 24 15 30 15",
        fill: None,
        stroke: Some((Ink::Dark, 0.7)),
        dash: true,
    },
];

pub const RAVENCLAW: &[Mark] = &[
    Mark {
        d: "M30 20C34 20 36 24 36 29L35 44 30 52 25 44 24 29C24 24 26 20 30 20Z",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M33 27C38 20 44 14 54 8 56 13 57 18 56 24 52 24 50 27 51 31 47 31 45 34 46 38 42 38 38 39 35 40Z",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M27 27C22 20 16 14 6 8 4 13 3 18 4 24 8 24 10 27 9 31 13 31 15 34 14 38 18 38 22 39 25 40Z",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M30 49L24 62 28 60 30 64 32 60 36 62Z",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M28 45L24 50 21 49M32 45L36 50 39 49",
        fill: None,
        stroke: Some((Ink::Metal, 1.6)),
        dash: false,
    },
    Mark {
        d: "M30 9C34 9 37 12 37 16 37 19 35 21 32 22L30 23 28 22C26 21 25 18 26 15 26 12 27 9 30 9Z",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M27 12C24 12 21 14 20 17 22 17 24 16 26 17Z",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "O28 14.0 0.9",
        fill: Some(Ink::Dark),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M46 14L41 22M51 13L44 24M42 30L38 33M14 14L19 22M9 13L16 24M18 30L22 33M28 30L30 34 32 30M27 37L30 41 33 37",
        fill: None,
        stroke: Some((Ink::Dark, 0.7)),
        dash: false,
    },
];

pub const HUFFLEPUFF: &[Mark] = &[
    Mark {
        d: "M50 40C50 31 42 26 32 26 24 26 19 28 15 31 11 32 6 35 4 38 7 40 11 41 15 42 18 46 24 49 32 49 42 49 50 47 50 40Z",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M50 38C53 38 56 40 57 42 54 42 52 41 50 41Z",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M18 46L16 56 21 56 22 48ZM26 48L26 57 31 57 31 49ZM38 48L40 57 45 57 43 47ZM45 46L49 55 53 54 48 44Z",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M14 31C17 30 22 30 27 32 22 33 17 34 13 36 11 36 9 37 7 37 9 34 11 32 14 31Z",
        fill: Some(Ink::Cloth),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "M15 35C19 35 24 34 30 33",
        fill: None,
        stroke: Some((Ink::Cloth, 1.6)),
        dash: false,
    },
    Mark {
        d: "M15 28.5L17 25 19 29",
        fill: Some(Ink::Metal),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "O11.6 36.6 0.9",
        fill: Some(Ink::Cloth),
        stroke: None,
        dash: false,
    },
    Mark {
        d: "O4.6 38.0 1.2",
        fill: Some(Ink::Dark),
        stroke: None,
        dash: false,
    },
];
