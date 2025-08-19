use std::collections::HashMap;
use std::sync::LazyLock;

use anyhow::{anyhow, bail, Context as _, Result};
use crate::output::{print_chain, print_ingredient};
use crate::{find_ingredient_in_recipe, find_ingredient_name, find_recipe, types::*};

use regex::Regex;

macro_rules! re {
    ($name:ident, $pattern:literal) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new($pattern).unwrap()
        });
    }
}

re!(RE_COMMENT, r"^#\s*(.+)$");
re!(RE_GROUP, r"^group\s+(.+)$");
re!(RE_MINE, r"^mine\s+([\d|\.]+)\s+(.+)$");
re!(RE_ALL_INTO, r"^all\s+(.+)\s+into\s+(.+)$");
re!(RE_USE_INTO, r"^use\s+([\d|\.]+)\s+(.+)\s+into\s+(.+)$");

#[allow(unused)]
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

#[derive(Debug, Default)]
pub struct Group {
    pub name: String,
    pub inputs: Vec<Ingredient>,
    pub outputs: Vec<Ingredient>,
    pub recipes: Vec<(f64, Recipe)>,
}

impl Group {
    pub fn balances(&self) -> Vec<Ingredient> {
        let mut b = Vec::new();
        for i in self.inputs.iter() { i.merge_with(&mut b); }
        for i in self.outputs.iter() { i.neg().merge_with(&mut b); }
        for (s, r) in self.recipes.iter() {
            for i in r.outputs() { i.scale(*s).merge_with(&mut b); }
            for i in r.inputs() { i.scale(*s).neg().merge_with(&mut b); }
        }
        b
    }
}

#[derive(Debug, Default)]
pub struct ChainState {
    pub groups: HashMap<String, Group>,
    pub current_group: Option<String>,
}

impl ChainState {
    pub fn set_or_make_group(&mut self, group: &str) {
        let mut g = Group::default();
        g.name = group.to_string();
        self.groups.entry(group.to_string()).or_insert(g);
        self.current_group = Some(group.to_string());
    }

    pub fn group(&mut self) -> &mut Group {
        let current_group = self.current_group.as_ref().expect("Must have a current group");
        self.groups.get_mut(current_group).expect("Could not get current group")
    }
}

pub fn process_chain(_state: State, chain: Vec<String>) -> Result<()> {
    let chain: Vec<(String, Result<Action>)> = chain.iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| (s.to_string(), Action::parse(s)))
        .collect();

    let mut any_err = false;
    for (l, a) in chain.iter().filter(|(_, a)| a.is_err()) {
        any_err = true;
        eprintln!("Parse error `{:?}` on line:\n{}", a, l);
    }
    if any_err { bail!("Could not parse chain file."); }

    let mut state = ChainState::default();

    let add_recipe = |state: &mut ChainState, ingredient: Ingredient, recipe: Recipe, use_ratio: f64| -> Result<()> {
        let g = state.group();
        let b = g.balances();
        let ingredient_balance = get_ingredient_from(&ingredient, b.iter())
            .with_context(|| format!("Could not get ingredient {} from balance:\n{:?}", ingredient.part, b))?;
        if ingredient_balance.quantity <= 0. {
            panic!("Can't take all from an ingredient with 0 or less balance.");
        }
        let recipe_ingredient = get_ingredient_from(&ingredient, recipe.inputs())
            .with_context(|| format!(
                "Could not get ingredient {} from recipe{} with ingredients:\n{:?}",
                ingredient.part,
                recipe.name,
                recipe.inputs().collect::<Vec<_>>()
            ))?;
        let ratio = (ingredient_balance.quantity * use_ratio) / recipe_ingredient.quantity;
        g.recipes.push((ratio, recipe));
        Ok(())
    };

    for (_l, a) in chain.into_iter() {
        let a = a.expect("Already errored out if anything went wrong.");
        match a {
            Action::Comment(_) => (),
            Action::Group { name } => { state.set_or_make_group(&name) },
            Action::Mine { ingredient } => {
                ingredient.merge_with(&mut state.group().inputs);
            },
            Action::AllInto { ingredient, recipe } => {
                add_recipe(&mut state, ingredient, recipe, 1.0)?;
            },
            Action::Use { fraction, ingredient, recipe } => {
                add_recipe(&mut state, ingredient, recipe, fraction)?;
            },
            Action::Unknown(x) => panic!("Encountered unknown directive {x}"),
        }
        // print_chain(&state); // For debug
    }

    print_chain(&state);
    Ok(())
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
            return Ok(Action::Mine{ ingredient: parse_ingredient(&caps[2], Some(&caps[1]), None)? });
        }
        if let Some(caps) = RE_ALL_INTO.captures(v) {
            let r = parse_recipe(&caps[2])?;
            return Ok(Action::AllInto {
                ingredient: parse_ingredient(&caps[1], None, Some(&r))?,
                recipe: r,
            });
        }
        if let Some(caps) = RE_USE_INTO.captures(v) {
            let r = parse_recipe(&caps[3])?;
            return Ok(Action::Use {
                fraction: parse_float(&caps[1])?,
                ingredient: parse_ingredient(&caps[2], None, Some(&r))?,
                recipe: r,
            });
        }

        Err(anyhow!("Could not parse chain command: {}", v))
        // Ok(Action::Unknown(v.into()))
    }
}

fn parse_float(n: &str) -> Result<f64> {
    Ok(n.parse::<f64>()?)
}

fn parse_ingredient(part: &str, number: Option<&str>, recipe: Option<&Recipe>) -> Result<Ingredient> {
    let amount = if let Some(n) = number {
        parse_float(n)?
    } else {
        0.0
    };
    let i = match recipe {
        Some(r) => find_ingredient_in_recipe(r, part)?.part.as_str(),
        None => find_ingredient_name(part)?,
    };
    Ok(Ingredient {
        part: i.to_string(),
        quantity: amount,
    })
}

fn parse_recipe(name: &str) -> Result<Recipe> {
    Ok(find_recipe(name)?.clone())
}

fn get_ingredient_from<'a, 'b>(
    query: &'a Ingredient,
    mut collection: impl Iterator<Item=&'b Ingredient>,
) -> Result<&'b Ingredient> {
    collection.find(|i| i.part == query.part)
        .ok_or(anyhow!("Could not find ingredient of type {}", query.part))
}
