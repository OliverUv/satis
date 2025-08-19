use anyhow::{anyhow, Result};
use crate::types::*;

#[derive(Debug, Clone)]
enum Actions {
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
}

pub fn process_chain(_state: State, all_recipes: RecipeMap, chain: Vec<String>) -> Result<()> {
    let chain: Vec<String> = chain.iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.into())
        .collect();
    for l in chain {
        println!("{}", l);
    }



    todo!()
}

impl From<String> for Actions {
    fn from(value: String) -> Self {
        todo!()
    }
}
