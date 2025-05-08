use anyhow::Result;
use chrono::{Datelike, NaiveDate};
use crossterm::event::{Event, KeyCode};
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Clone, Copy, PartialEq)]
pub enum DatePart {
    Year,
    Month,
    Day,
}

pub struct DateInputState {
    pub date: NaiveDate,
    pub editing: bool,
    pub date_part: DatePart,
    pub current_date_input: String,
}

impl DateInputState {
    pub fn new(date: NaiveDate) -> Self {
        Self {
            date,
            editing: false,
            date_part: DatePart::Year,
            current_date_input: String::new(),
        }
    }

    pub fn toggle_editing(&mut self) {
        self.editing = !self.editing;
        if self.editing {
            self.date_part = DatePart::Year;
            self.current_date_input.clear();
        }
    }

    pub fn next_date_part(&mut self) {
        self.date_part = match self.date_part {
            DatePart::Year => DatePart::Month,
            DatePart::Month => DatePart::Day,
            DatePart::Day => DatePart::Year,
        };
        self.current_date_input.clear();
    }

    pub fn previous_date_part(&mut self) {
        self.date_part = match self.date_part {
            DatePart::Year => DatePart::Day,
            DatePart::Month => DatePart::Year,
            DatePart::Day => DatePart::Month,
        };
        self.current_date_input.clear();
    }

    pub fn handle_input(&mut self, key: KeyCode) {
        if !self.editing {
            return;
        }

        match key {
            KeyCode::Char(c) if c.is_digit(10) => {
                let year = self.date.year();
                let month = self.date.month();
                let day = self.date.day();

                match self.date_part {
                    DatePart::Year => {
                        self.current_date_input.push(c);
                        if self.current_date_input.len() == 4 {
                            if let Ok(new_year) = self.current_date_input.parse::<i32>() {
                                if new_year >= 1900 && new_year <= 2100 {
                                    if let Some(new_date) = NaiveDate::from_ymd_opt(new_year, month, day) {
                                        self.date = new_date;
                                    }
                                }
                            }
                            self.current_date_input.clear();
                        } else if self.current_date_input.len() > 4 {
                            self.current_date_input = self.current_date_input.chars().rev().take(4).collect::<String>().chars().rev().collect();
                        }
                    }
                    DatePart::Month => {
                        self.current_date_input.push(c);
                        if self.current_date_input.len() == 2 {
                            if let Ok(new_month) = self.current_date_input.parse::<u32>() {
                                if new_month >= 1 && new_month <= 12 {
                                    if let Some(new_date) = NaiveDate::from_ymd_opt(year, new_month, day) {
                                        self.date = new_date;
                                    }
                                }
                            }
                            self.current_date_input.clear();
                        }
                    }
                    DatePart::Day => {
                        self.current_date_input.push(c);
                        if self.current_date_input.len() == 2 {
                            if let Ok(new_day) = self.current_date_input.parse::<u32>() {
                                let max_day = days_in_month(year, month);
                                if new_day >= 1 && new_day <= max_day {
                                    if let Some(new_date) = NaiveDate::from_ymd_opt(year, month, new_day) {
                                        self.date = new_date;
                                    }
                                }
                            }
                            self.current_date_input.clear();
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                self.current_date_input.pop();
            }
            KeyCode::Right => self.next_date_part(),
            KeyCode::Left => self.previous_date_part(),
            _ => {}
        }
    }

    pub fn get_display_string(&self) -> String {
        let date_str = format!("{}", self.date.format("%Y-%m-%d"));
        let parts: Vec<&str> = date_str.split('-').collect();
        
        if parts.len() == 3 {
            let (year, month, day) = (parts[0], parts[1], parts[2]);
            if self.editing {
                let current_input = if !self.current_date_input.is_empty() {
                    format!("[{}]", self.current_date_input)
                } else {
                    match self.date_part {
                        DatePart::Year => "[YYYY]".to_string(),
                        DatePart::Month => "[MM]".to_string(),
                        DatePart::Day => "[DD]".to_string(),
                    }
                };
                
                match self.date_part {
                    DatePart::Year => format!("{}{}-{}-{}", year, current_input, month, day),
                    DatePart::Month => format!("{}-{}{}-{}", year, month, current_input, day),
                    DatePart::Day => format!("{}-{}-{}{}", year, month, day, current_input),
                }
            } else {
                date_str
            }
        } else {
            date_str
        }
    }
}

// Helper function to get the number of days in a month
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            // February: handle leap years
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30, // Default case (shouldn't happen with valid input)
    }
} 