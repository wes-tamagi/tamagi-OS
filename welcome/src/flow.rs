//! The pages and the order they are walked in.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Welcome,
    Profile,
    Browser,
    /// Only for "choose item by item".
    Catalog,
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
            Page::Profile => "Profile",
            Page::Browser => "Browser",
            Page::Catalog => "Catalog",
            Page::Disk => "Encrypted disk",
            Page::Review => "Review",
            Page::Installing => "Installing",
            Page::Done => "Done",
        }
    }
}

/// The pages of this run, in order.
pub fn pages(item_by_item: bool, disk_question: bool) -> Vec<Page> {
    let mut pages = vec![Page::Welcome, Page::Profile, Page::Browser];
    if item_by_item {
        pages.push(Page::Catalog);
    }
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
        assert_eq!(pages(false, false), [Welcome, Profile, Browser, Review, Installing, Done]);
        assert_eq!(pages(true, false), [Welcome, Profile, Browser, Catalog, Review, Installing, Done]);
        assert_eq!(pages(false, true), [Welcome, Profile, Browser, Disk, Review, Installing, Done]);
        assert_eq!(pages(true, true), [Welcome, Profile, Browser, Catalog, Disk, Review, Installing, Done]);
    }

    #[test]
    fn forward() {
        let p = pages(true, true);
        assert_eq!(next(&p, Welcome), Some(Profile));
        assert_eq!(next(&p, Browser), Some(Catalog));
        assert_eq!(next(&p, Catalog), Some(Disk));
        assert_eq!(next(&p, Disk), Some(Review));
        assert_eq!(next(&p, Review), None);
        assert_eq!(next(&p, Done), None);
        let short = pages(false, false);
        assert_eq!(next(&short, Browser), Some(Review));
    }

    #[test]
    fn backward() {
        let p = pages(true, false);
        assert_eq!(back(&p, Welcome), None);
        assert_eq!(back(&p, Review), Some(Catalog));
        assert_eq!(back(&p, Catalog), Some(Browser));
        assert_eq!(back(&p, Installing), None);
        assert_eq!(back(&p, Done), None);
        // Changing the profile away from "item by item" removes the catalog page.
        let short = pages(false, false);
        assert_eq!(back(&short, Review), Some(Browser));
    }
}
