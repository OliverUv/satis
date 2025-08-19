use std::collections::HashSet;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::LazyLock;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

pub mod types;
use types::*;
pub mod output;
use output::*;
pub mod chain;
use chain::*;
pub mod import;
use import::get_all_recipes;

pub static ALL_RECIPES: LazyLock<RecipeMap> = LazyLock::new(|| {
    let r = get_all_recipes();
    match r {
        Err(e) => { panic!("Could not parse recipes: {}", e); }
        Ok(r) => r,
    }
});

pub static ALL_INGREDIENTS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    let mut set = HashSet::new();
    for (_name, r) in ALL_RECIPES.iter() {
        for i in r.ingredients() {
            set.insert(i.part.clone());
        }
    }
    set
});


#[derive(Parser)]
#[command(name = "satis")]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a blueprint
    Bp{recipe: String},
    /// See a recipe given a certain amount of an ingredient
    Mult{recipe: String, ingredient: String, amount: f64},
    /// Show recipe
    Show{recipe: String},
    /// Find all recipes that produce the given ingredient
    Find{ingredient: String},
    /// Do stuff with production chains
    Chain{file_path: PathBuf},
}

fn main() -> Result<()> {
    let state = State::default();
    let cli = Cli::parse();
    match &cli.command {
        Command::Bp{recipe} => suggest_blueprint(state, recipe.as_str())?,
        Command::Mult{recipe, ingredient, amount} => {
            mult(state, recipe.as_str(), ingredient.as_str(), *amount)?;
        }
        Command::Show{recipe} => {
            find_recipe(recipe)?.print();
        }
        Command::Find{ingredient} => {
            let i = find_ingredient_name(ingredient)?;
            ALL_RECIPES.iter()
                .map(|(_, r)| r)
                .filter(|r| r.outputs().any(|o| o.part == i))
                .for_each(|r| {
                    println!("=========");
                    r.print();
                    println!("");
                })
        },
        Command::Chain{file_path} => {
            println!("\nUsing chain from: {file_path:?}\n");
            let chain = read_to_string(file_path)?
                .lines()
                .map(|l| l.into())
                .collect();
            process_chain(state, chain)?;
        }
    }
    Ok(())
}

fn find_recipe<'a, 'b>(recipe_query: &'b str) -> Result<&'a Recipe> {
    // First try to find exact match
    let exact = ALL_RECIPES.keys().find(|k| k.to_lowercase() == recipe_query.to_lowercase());
    if let Some(e) = exact { return Ok(ALL_RECIPES.get(e).expect("Already verified key is in dict")); }

    // Otherwise fuzz
    let matcher = SkimMatcherV2::default();
    let mut fuzz: Vec<(&str, i64)> = ALL_RECIPES.keys()
        .map(String::as_str)
        .map(|key| (key, matcher.fuzzy_match(key, recipe_query.to_lowercase().as_str())))
        .filter(|(_key, score)| score.is_some())
        .map(|(key, score)| (key, score.expect("Filtered out Nones already")))
        .collect();
    fuzz.sort_by_key(|(_key, score)| *score);
    let best_match_key = fuzz.last().ok_or(anyhow!("Could not find recipe: {recipe_query}"))?.0;
    ALL_RECIPES.get(best_match_key).ok_or(anyhow!("Could not find recipe: {best_match_key}"))
}

fn find_ingredient_in_recipe<'a, 'b>(recipe: &'a Recipe, ingredient_query: &'b str) -> Result<&'a Ingredient> {
    // First try to find exact match
    let exact = recipe.ingredients().find(|i| i.same_type(ingredient_query));
    if let Some(e) = exact { return Ok(e); }

    // Otherwise fuzz
    let matcher = SkimMatcherV2::default();
    let mut fuzz: Vec<(&Ingredient, i64)> = recipe.ingredients()
        .map(|i| (i, matcher.fuzzy_match(i.part.as_str(), ingredient_query.to_lowercase().as_str())))
        .filter(|(_i, score)| score.is_some())
        .map(|(i, score)| (i, score.expect("Filtered out Nones already")))
        .collect();
    fuzz.sort_by_key(|(_i, score)| *score);
    let best_match_ingredient = fuzz.last().ok_or(anyhow!("Could not find ingredient {ingredient_query} in {recipe:?}"))?.0;
    Ok(best_match_ingredient)
}

fn find_ingredient_name<'a, 'b>(ingredient_query:&'b str) -> Result<&'a str> {
    // First try to find exact match
    let exact = ALL_INGREDIENTS.iter().find(|i| i.to_lowercase() == ingredient_query.to_lowercase());
    if let Some(e) = exact { return Ok(e); }

    // Otherwise fuzz
    let matcher = SkimMatcherV2::default();
    let mut fuzz: Vec<(&String, i64)> = ALL_INGREDIENTS.iter()
        .map(|i| (i, matcher.fuzzy_match(i.as_str(), ingredient_query.to_lowercase().as_str())))
        .filter(|(_i, score)| score.is_some())
        .map(|(i, score)| (i, score.expect("Filtered out Nones already")))
        .collect();
    fuzz.sort_by_key(|(_i, score)| *score);
    let best_match_ingredient = fuzz.last().ok_or(anyhow!("Could not find ingredient name: {ingredient_query}"))?.0;
    Ok(best_match_ingredient)
}

fn suggest_blueprint(state: State, recipe: &str) -> Result<()> {
    let r = find_recipe(recipe)?;
    r.print_blueprint_suggestion(&state)?;
    Ok(())
}

fn mult(_state: State, recipe: &str, ingredient: &str, amount: f64) -> Result<()> {
    let r = find_recipe(recipe)?;

    let i = find_ingredient_in_recipe(r, ingredient)?;
    let factor = amount/i.quantity;

    println!("\nStandard Recipe: {}\n", r.name);
    println!("Out:");
    r.outputs().for_each(|i| print_ingredient(i, None));
    println!("In:");
    r.inputs().for_each(|i| print_ingredient(i, None));

    println!("\n{}  (x{:.4})  [{} = {}]\n", r.name, factor, i.part, amount);
    println!("Out:");
    r.outputs().for_each(|i| print_ingredient(i, Some(factor)));
    println!("In:");
    r.inputs().for_each(|i| print_ingredient(i, Some(factor)));

    Ok(())
}

