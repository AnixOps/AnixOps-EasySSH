#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breakpoint {
    Desktop,
    Tablet,
    Mobile,
}

impl Breakpoint {
    pub fn for_width(width: f32) -> Self {
        if width >= 1200.0 {
            Self::Desktop
        } else if width >= 800.0 {
            Self::Tablet
        } else {
            Self::Mobile
        }
    }

    #[cfg(feature = "ui-test")]
    pub fn name(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Tablet => "tablet",
            Self::Mobile => "mobile",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakpoints_match_the_shell_contract() {
        assert_eq!(Breakpoint::for_width(1200.0), Breakpoint::Desktop);
        assert_eq!(Breakpoint::for_width(800.0), Breakpoint::Tablet);
        assert_eq!(Breakpoint::for_width(799.0), Breakpoint::Mobile);
    }
}
