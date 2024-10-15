use anyhow::anyhow;
use clap::{Parser, Subcommand};

pub mod types;
use types::*;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Show{recipe: String},
    // List,
}

fn main() -> Result<(), anyhow::Error> {
    // println!("Reading recipes");
    let all_recipes = std::fs::read_to_string("./all_recipes.toml")?;
    let all_recipes = toml::from_str::<Recipes>(&all_recipes)?.recipes;

    let state = State::default();

    let cli = Cli::parse();
    match &cli.command {
        Command::Show{recipe} => show(state, all_recipes, recipe.as_str())?,
    }

    Ok(())
}

fn show(state: State, all_recipes: RecipeMap, recipe: &str) -> Result<(), anyhow::Error> {
    let r = all_recipes.get(recipe)
        .ok_or(anyhow!("No such recipe: {}", &recipe))?;
    r.print_calc(&state)?;
    Ok(())
}
