use std::collections::HashSet;
use std::path::PathBuf;
use std::fs::read_to_string;

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
    let all_recipes = get_all_recipes()?;
    let all_ingredients = {
        let mut set = HashSet::new();
        for (_name, r) in all_recipes.iter() {
            for i in r.ingredients() {
                set.insert(i.part.to_lowercase());
            }
        }
        set
    };

    let state = State::default();
    let cli = Cli::parse();
    match &cli.command {
        Command::Bp{recipe} => suggest_blueprint(state, all_recipes, recipe.as_str())?,
        Command::Mult{recipe, ingredient, amount} => {
            mult(state, all_recipes, recipe.as_str(), ingredient.as_str(), *amount)?;
        }
        Command::Show{recipe} => {
            let r = find_recipe(&all_recipes, recipe)?;
            r.print();
        }
        Command::Find{ingredient} => {
            let i = find_ingredient(&all_ingredients, ingredient)?;
            all_recipes.iter()
                .map(|(_, r)| r)
                .filter(|r| r.outputs().any(|o| o.part.to_lowercase() == i))
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
            process_chain(state, all_recipes, chain)?;
        }
    }
    Ok(())
}

fn find_recipe<'a, 'b>(all_recipes: &'a RecipeMap, recipe_query: &'b str) -> Result<&'a Recipe> {
    let matcher = SkimMatcherV2::default();
    let mut fuzz: Vec<(&str, i64)> = all_recipes.keys()
        .map(String::as_str)
        .map(|key| (key, matcher.fuzzy_match(key, recipe_query)))
        .filter(|(_key, score)| score.is_some())
        .map(|(key, score)| (key, score.expect("Filtered out Nones already")))
        .collect();
    fuzz.sort_by_key(|(_key, score)| *score);
    let best_match_key = fuzz.last().ok_or(anyhow!("Could not find recipe: {recipe_query}"))?.0;
    all_recipes.get(best_match_key).ok_or(anyhow!("Could not find recipe: {best_match_key}"))
}

fn find_ingredient_in_recipe<'a, 'b>(recipe: &'a Recipe, ingredient_query: &'b str) -> Result<&'a Ingredient> {
    let matcher = SkimMatcherV2::default();
    let mut fuzz: Vec<(&Ingredient, i64)> = recipe.ingredients()
        .map(|i| (i, matcher.fuzzy_match(i.part.as_str(), ingredient_query)))
        .filter(|(_i, score)| score.is_some())
        .map(|(i, score)| (i, score.expect("Filtered out Nones already")))
        .collect();
    fuzz.sort_by_key(|(_i, score)| *score);
    let best_match_ingredient = fuzz.last().ok_or(anyhow!("Could not find ingredient: {ingredient_query}"))?.0;
    Ok(best_match_ingredient)
}

fn find_ingredient<'a, 'b>(all_ingredients: &'a HashSet<String>, ingredient_query:&'b str) -> Result<&'a str> {
    let matcher = SkimMatcherV2::default();
    let mut fuzz: Vec<(&String, i64)> = all_ingredients.iter()
        .map(|i| (i, matcher.fuzzy_match(i.as_str(), ingredient_query)))
        .filter(|(_i, score)| score.is_some())
        .map(|(i, score)| (i, score.expect("Filtered out Nones already")))
        .collect();
    fuzz.sort_by_key(|(_i, score)| *score);
    let best_match_ingredient = fuzz.last().ok_or(anyhow!("Could not find ingredient: {ingredient_query}"))?.0;
    Ok(best_match_ingredient)
}

fn suggest_blueprint(state: State, all_recipes: RecipeMap, recipe: &str) -> Result<()> {
    let r = find_recipe(&all_recipes, recipe)?;
    r.print_blueprint_suggestion(&state)?;
    Ok(())
}

fn mult(_state: State, all_recipes: RecipeMap, recipe: &str, ingredient: &str, amount: f64) -> Result<()> {
    let r = find_recipe(&all_recipes, recipe)?;

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

