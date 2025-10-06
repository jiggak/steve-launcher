/*
 * Steve Launcher - A Minecraft Launcher
 * Copyright (C) 2025 Josh Kropf <josh@slashdev.ca>
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

use anyhow::Result;
use reqwest::{Method, RequestBuilder};

pub trait ApiClient {
    fn request(&self, method: Method, uri: &str) -> RequestBuilder;

    async fn get<T>(&self, url: &str) -> Result<T>
        where T: serde::de::DeserializeOwned
    {
        Ok(self.request(Method::GET, url)
            .send().await?
            .error_for_status()?
            .json::<T>().await?)
    }

    async fn post<T, R>(&self, url: &str, body: &R) -> Result<T>
        where T: serde::de::DeserializeOwned, R: serde::Serialize
    {
        Ok(self.request(Method::POST, url)
            .json(body)
            .send().await?
            .error_for_status()?
            .json::<T>().await?)
    }
}
