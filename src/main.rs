use color_eyre::eyre::Result;
use rand::Rng;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{self, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    prelude::*,
    widgets::*,
};
use std::io::{self, stdout};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn is_opposite(&self, other: &Direction) -> bool {
        matches!(
            (self, other),
            (Direction::Up, Direction::Down)
                | (Direction::Down, Direction::Up)
                | (Direction::Left, Direction::Right)
                | (Direction::Right, Direction::Left)
        )
    }
}

struct GameState {
    snake: Vec<(u16, u16)>,
    food: Option<(u16, u16)>,
    direction: Direction,
    last_direction: Direction,
    game_over: bool,
    score: u32,
    area: Rect,
}

impl GameState {
    fn new(area: Rect) -> Self {
        let mut state = Self {
            snake: vec![(area.width / 2, area.height / 2)],
            food: None,
            direction: Direction::Right,
            last_direction: Direction::Right,
            game_over: false,
            score: 0,
            area,
        };
        state.spawn_food();
        state
    }

    fn spawn_food(&mut self) {
        let mut rng = rand::thread_rng();
        loop {
            let x = rng.gen_range(1..self.area.width - 1);
            let y = rng.gen_range(1..self.area.height - 1);
            if !self.snake.contains(&(x, y)) {
                self.food = Some((x, y));
                break;
            }
        }
    }

    fn update(&mut self) {
        if self.game_over {
            return;
        }

        let (head_x, head_y) = self.snake[0];
        let new_head = match self.direction {
            Direction::Up => (head_x, head_y.saturating_sub(1)),
            Direction::Down => (head_x, head_y + 1),
            Direction::Left => (head_x.saturating_sub(1), head_y),
            Direction::Right => (head_x + 1, head_y),
        };

        if new_head.0 < 1
            || new_head.0 >= self.area.width - 1
            || new_head.1 < 1
            || new_head.1 >= self.area.height - 1
        {
            self.game_over = true;
            return;
        }

        if self.snake.contains(&new_head) {
            self.game_over = true;
            return;
        }

        if let Some(food) = self.food {
            if new_head == food {
                self.score += 1;
                self.snake.insert(0, new_head);
                self.spawn_food();
            } else {
                self.snake.insert(0, new_head);
                self.snake.pop();
            }
        } else {
            self.snake.insert(0, new_head);
            self.snake.pop();
        }

        self.last_direction = self.direction;
    }

    fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up if !self.last_direction.is_opposite(&Direction::Up) => {
                self.direction = Direction::Up;
            }
            KeyCode::Down if !self.last_direction.is_opposite(&Direction::Down) => {
                self.direction = Direction::Down;
            }
            KeyCode::Left if !self.last_direction.is_opposite(&Direction::Left) => {
                self.direction = Direction::Left;
            }
            KeyCode::Right if !self.last_direction.is_opposite(&Direction::Right) => {
                self.direction = Direction::Right;
            }
            _ => {}
        }
    }
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn ui(frame: &mut Frame, game_state: &GameState) {
    let area = frame.area();

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Snake Game")
        .title_alignment(Alignment::Center);
    frame.render_widget(block, area);

    for &(x, y) in &game_state.snake {
        let segment = Paragraph::new("█").style(Style::default().fg(Color::Green));
        frame.render_widget(segment, Rect::new(x, y, 1, 1));
    }

    if let Some((x, y)) = game_state.food {
        let food = Paragraph::new("●").style(Style::default().fg(Color::Red));
        frame.render_widget(food, Rect::new(x, y, 1, 1));
    }

    let score = Paragraph::new(format!("Score: {}", game_state.score))
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);
    frame.render_widget(score, Rect::new(area.x, area.y + 1, area.width, 1));

    let controls = Paragraph::new("Arrow keys: Move | Q: Quit")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    frame.render_widget(
        controls,
        Rect::new(area.x, area.bottom() - 2, area.width, 1),
    );

    if game_state.game_over {
        let msg = Paragraph::new("GAME OVER! Press any key to exit.")
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        frame.render_widget(msg, area);
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = init_terminal()?;
    let mut game_state = GameState::new(terminal.size()?.into());
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    loop {
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_default();

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                            game_state.handle_key(key.code);
                        }
                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            game_state.update();
            last_tick = Instant::now();
        }

        terminal.draw(|frame| ui(frame, &game_state))?;

        if game_state.game_over {
            event::read()?;
            break;
        }
    }

    restore_terminal(&mut terminal)?;
    Ok(())
}
