use super::*;

pub(crate) struct HelpView {
  message_backup: Option<String>,
  visible: bool,
}

impl HelpView {
  pub(crate) fn draw(&self, frame: &mut Frame) {
    if !self.visible {
      return;
    }

    let area = Self::help_area(frame.area());

    frame.render_widget(Clear, area);

    let help = Paragraph::new(HELP_TEXT)
      .block(Block::default().title(HELP_TITLE).borders(Borders::ALL))
      .wrap(Wrap { trim: true });

    frame.render_widget(help, area);
  }

  pub(crate) fn handle_key(key: KeyEvent) -> Command {
    match key.code {
      KeyCode::Char('?') | KeyCode::Esc => Command::HideHelp,
      KeyCode::Char('q' | 'Q') => Command::Quit,
      _ => Command::None,
    }
  }

  fn help_area(area: Rect) -> Rect {
    fn saturating_usize_to_u16(value: usize) -> u16 {
      u16::try_from(value).unwrap_or(u16::MAX)
    }

    let (line_count, max_line_width) =
      HELP_TEXT
        .lines()
        .fold((0usize, 0usize), |(count, width), line| {
          let updated_count = count.saturating_add(1);
          let line_width = line.chars().count();

          (updated_count, width.max(line_width))
        });

    let desired_width =
      saturating_usize_to_u16(max_line_width.saturating_add(2)).max(1);

    let desired_height =
      saturating_usize_to_u16(line_count.saturating_add(2)).max(1);

    let available_width = area.width.saturating_sub(2).max(1);
    let available_height = area.height.saturating_sub(2).max(1);

    let width = available_width.clamp(1, desired_width).min(area.width);
    let height = available_height.clamp(1, desired_height).min(area.height);

    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    Rect::new(x, y, width, height)
  }

  pub(crate) fn hide(&mut self, message: &mut String) {
    if !self.visible {
      return;
    }

    *message = self
      .message_backup
      .take()
      .unwrap_or_else(|| LIST_STATUS.into());

    self.visible = false;
  }

  pub(crate) fn is_visible(&self) -> bool {
    self.visible
  }

  pub(crate) fn new() -> Self {
    Self {
      message_backup: None,
      visible: false,
    }
  }

  pub(crate) fn show(&mut self, message: &mut String) {
    if self.visible {
      return;
    }

    self.message_backup = Some(message.clone());

    *message = HELP_STATUS.into();

    self.visible = true;
  }
}
