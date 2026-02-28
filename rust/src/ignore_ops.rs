// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! A tiny module for specifying what shall be ignored during simulation

#[derive(Debug, clap::ValueEnum, Clone, Copy)]
pub enum IgnoreOps {
    Boxes,
    Nothing,
}
