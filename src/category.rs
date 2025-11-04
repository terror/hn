#[derive(Clone, Copy)]
pub enum CategoryKind {
  Bookmarks,
  Comments,
  Search,
  Stories(&'static str),
}

#[derive(Clone, Copy)]
pub struct Category {
  pub kind: CategoryKind,
  pub label: &'static str,
}

impl Category {
  pub fn all() -> &'static [Category] {
    &[
      Category {
        label: "top",
        kind: CategoryKind::Stories("topstories"),
      },
      Category {
        label: "new",
        kind: CategoryKind::Stories("newstories"),
      },
      Category {
        label: "past",
        kind: CategoryKind::Stories("topstories"),
      },
      Category {
        label: "comments",
        kind: CategoryKind::Comments,
      },
      Category {
        label: "ask",
        kind: CategoryKind::Stories("askstories"),
      },
      Category {
        label: "show",
        kind: CategoryKind::Stories("showstories"),
      },
      Category {
        label: "jobs",
        kind: CategoryKind::Stories("jobstories"),
      },
    ]
  }
}
