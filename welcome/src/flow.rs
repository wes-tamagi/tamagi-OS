//! The pages and the order they are walked in.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Welcome,
    /// The basics: a short list to tick.
    Apps,
    Browser,
    /// Only with an encrypted disk and a TPM not used yet.
    Disk,
    Review,
    Installing,
    Done,
}

impl Page {
    /// Short name in the step indicator (English: a translation key).
    pub fn label(self) -> &'static str {
        match self {
            Page::Welcome => "Welcome",
            Page::Apps => "Apps",
            Page::Browser => "Browser",
            Page::Disk => "Encrypted disk",
            Page::Review => "Review",
            Page::Installing => "Installing",
            Page::Done => "Done",
        }
    }
}

/// The pages of this run, in order.
pub fn pages(disk_question: bool) -> Vec<Page> {
    let mut pages = vec![Page::Welcome, Page::Apps, Page::Browser];
    if disk_question {
        pages.push(Page::Disk);
    }
    pages.extend([Page::Review, Page::Installing, Page::Done]);
    pages
}

/// The page after `page` while choosing; None at the review (what comes next
/// is installing, which is not a simple step forward).
pub fn next(pages: &[Page], page: Page) -> Option<Page> {
    if matches!(page, Page::Review | Page::Installing | Page::Done) {
        return None;
    }
    let pos = pages.iter().position(|p| *p == page)?;
    pages.get(pos + 1).copied()
}

/// The page before `page`; None on the first one and once installing started.
pub fn back(pages: &[Page], page: Page) -> Option<Page> {
    if matches!(page, Page::Installing | Page::Done) {
        return None;
    }
    let pos = pages.iter().position(|p| *p == page)?;
    pos.checked_sub(1).and_then(|p| pages.get(p)).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use Page::*;

    #[test]
    fn page_lists() {
        assert_eq!(pages(false), [Welcome, Apps, Browser, Review, Installing, Done]);
        assert_eq!(pages(true), [Welcome, Apps, Browser, Disk, Review, Installing, Done]);
    }

    #[test]
    fn forward() {
        let p = pages(true);
        assert_eq!(next(&p, Welcome), Some(Apps));
        assert_eq!(next(&p, Apps), Some(Browser));
        assert_eq!(next(&p, Browser), Some(Disk));
        assert_eq!(next(&p, Disk), Some(Review));
        assert_eq!(next(&p, Review), None);
        assert_eq!(next(&p, Done), None);
        let short = pages(false);
        assert_eq!(next(&short, Browser), Some(Review));
    }

    #[test]
    fn backward() {
        let p = pages(false);
        assert_eq!(back(&p, Welcome), None);
        assert_eq!(back(&p, Review), Some(Browser));
        assert_eq!(back(&p, Browser), Some(Apps));
        assert_eq!(back(&p, Installing), None);
        assert_eq!(back(&p, Done), None);
        assert_eq!(back(&pages(true), Review), Some(Disk));
    }
}
