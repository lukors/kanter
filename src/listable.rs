use kanter_core::{
    node::{MixType, ResizeFilter, ResizePolicy},
    node_data::Size as TPSize,
    node_graph::SlotId,
};

pub(crate) trait Listable<T> {
    fn list() -> String;
    fn choose(i: usize) -> Option<T>;
}

impl Listable<Self> for ResizePolicy {
    fn list() -> String {
        let mut output = "## Resize policy\n".to_string();
        let entries = vec![
            "MostPixels".to_string(),
            "LeastPixels".to_string(),
            "LargestAxes".to_string(),
            "SmallestAxes".to_string(),
            "SpecificSlot".to_string(),
            "SpecificSize".to_string(),
        ];
        for (i, entry) in entries.iter().enumerate() {
            output = format!("{}{}: {}\n", output, i + 1, entry);
        }
        output
    }

    fn choose(i: usize) -> Option<Self> {
        const MAX_CHOICE: usize = 6;

        if i <= MAX_CHOICE {
            Some(match i {
                1 => Self::MostPixels,
                2 => Self::LeastPixels,
                3 => Self::LargestAxes,
                4 => Self::SmallestAxes,
                5 => Self::SpecificSlot(SlotId(0)),
                _ => Self::SpecificSize(TPSize::new(128, 128)),
            })
        } else {
            None
        }
    }
}

impl Listable<Self> for ResizeFilter {
    fn list() -> String {
        let mut output = "## Resize filter\n".to_string();
        let entries = vec!["Nearest".to_string(), "Triangle".to_string()];
        for (i, entry) in entries.iter().enumerate() {
            output = format!("{}{}: {}\n", output, i + 1, entry);
        }
        output
    }

    fn choose(i: usize) -> Option<Self> {
        const MAX_CHOICE: usize = 2;

        if i <= MAX_CHOICE {
            Some(match i {
                1 => Self::Nearest,
                _ => Self::Triangle,
            })
        } else {
            None
        }
    }
}

impl Listable<Self> for MixType {
    fn list() -> String {
        let mut output = "## Mix Type\n".to_string();
        let entries = vec![
            Self::Add.to_string(),
            Self::Subtract.to_string(),
            Self::Multiply.to_string(),
            Self::Divide.to_string(),
        ];
        for (i, entry) in entries.iter().enumerate() {
            output = format!("{}{}: {}\n", output, i + 1, entry);
        }
        output
    }

    fn choose(i: usize) -> Option<Self> {
        const MAX_CHOICE: usize = 4;

        if i <= MAX_CHOICE {
            Some(match i {
                1 => Self::Add,
                2 => Self::Subtract,
                3 => Self::Multiply,
                _ => Self::Divide,
            })
        } else {
            None
        }
    }
}
