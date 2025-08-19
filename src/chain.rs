use std::sync::LazyLock;

use anyhow::{anyhow, bail, Result};
use crate::{find_ingredient_name, types::*};

use regex::Regex;

macro_rules! re {
    ($name:ident, $pattern:literal) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new($pattern).unwrap()
        });
    }
}

re!(RE_COMMENT, r"^#\s*(.*)$");
re!(RE_GROUP, r"^group\s*(.*)$");
re!(RE_MINE, r"^mine\s*(\d*)\s*(.*)$");

#[derive(Debug, Clone)]
enum Action {
    Comment(String),
    Group { name: String },
    Mine { ingredient: Ingredient },
    // InFrom { ingredient: Ingredient, group: String },
    AllInto { ingredient: Ingredient, recipe: Recipe },
    Use {
        fraction: f64,
        ingredient: Ingredient,
        recipe: Recipe,
    },
    Unknown(String),
}

pub fn process_chain(_state: State, chain: Vec<String>) -> Result<()> {
    let chain: Vec<Result<Action>> = chain.iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(Action::parse)
        .collect();

    let mut any_err = false;
    for a in chain.iter().filter(|a| a.is_err()) {
        any_err = true;
        eprintln!("Could not parse: {:?}", a);
    }
    if any_err { bail!("Could not parse chain file."); }

    for a in chain.into_iter().filter_map(|a| a.ok()) {
        println!("{:?}", a);
    }

    todo!()
}

impl Action {
    fn parse(v: &str) -> Result<Self> {
        if let Some(caps) = RE_COMMENT.captures(v) {
            return Ok(Action::Comment(caps[1].into()));
        }
        if let Some(caps) = RE_GROUP.captures(v) {
            return Ok(Action::Group{name: caps[1].into()});
        }
        if let Some(caps) = RE_MINE.captures(v) {
            return Ok(Action::Mine{ ingredient: parse_ingredient(&caps[2], Some(&caps[1]))? });
        }

        // Err(anyhow!("Could not parse chain command: {}", v))
        Ok(Action::Unknown(v.into()))
    }
}

fn parse_ingredient(part: &str, number: Option<&str>) -> Result<Ingredient> {
    let amount = if let Some(n) = number {
        n.parse::<f64>()?
    } else {
        0.0
    };
    let i = find_ingredient_name(part)?;
    Ok(Ingredient {
        part: i.to_string(),
        quantity: amount,
    })
}
