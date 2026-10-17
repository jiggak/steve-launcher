/*
 * Steve Launcher - A Minecraft Launcher
 * Copyright (C) 2026 Josh Kropf <josh@slashdev.ca>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::time::{Duration, Instant};

pub struct Debouncer<T> {
    delay: Duration,
    pending: Option<(T, Instant)>,
}

impl<T> Debouncer<T> {
    pub fn new(delay: Duration) -> Self {
        Self { delay, pending: None }
    }

    /// Store the latest value and restart the timer.
    pub fn push(&mut self, value: T) {
        self.pending = Some((value, Instant::now() + self.delay));
    }

    /// Returns the value once, after the delay has elapsed with no new pushes.
    pub fn poll(&mut self) -> Option<T> {
        self.poll_at(Instant::now())
    }

    // Separate so tests can pass a fake `now` without sleeping.
    fn poll_at(&mut self, now: Instant) -> Option<T> {
        match &self.pending {
            Some((_, deadline)) if now >= *deadline => {
                self.pending.take().map(|(value, _)| value)
            }
            _ => None,
        }
    }
}
