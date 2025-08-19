use anyhow::anyhow;
use clap::{Parser, Subcommand};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

pub mod types;
use types::*;

pub mod import;
use import::get_all_recipes;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a blueprint
    Calc{recipe: String},
    /// See a recipe given a certain amount of an ingredient
    Mult{recipe: String, ingredient: String, amount: f64}
}

fn main() -> Result<(), anyhow::Error> {
    let all_recipes = get_all_recipes()?;
    let state = State::default();
    let cli = Cli::parse();
    match &cli.command {
        Command::Calc{recipe} => calc(state, all_recipes, recipe.as_str())?,
        Command::Mult{recipe, ingredient, amount} => {
            mult(state, all_recipes, recipe.as_str(), ingredient.as_str(), *amount)?;
        }
    }
    Ok(())
}

fn find_recipe<'a, 'b>(all_recipes: &'a RecipeMap, recipe_query: &'b str) -> Result<&'a Recipe, anyhow::Error> {
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

fn find_ingredient<'a, 'b>(recipe: &'a Recipe, ingredient_query: &'b str) -> Result<&'a Ingredient, anyhow::Error> {
    let matcher = SkimMatcherV2::default();
    let mut fuzz: Vec<(&Ingredient, i64)> = recipe.ingredients().into_iter()
        // .filter(|i| i.is_some())
        // .map(|i| i.as_ref().unwrap())
        .filter_map(|i| i.as_ref())
        .map(|i| (i, matcher.fuzzy_match(i.part.as_str(), ingredient_query)))
        .filter(|(_i, score)| score.is_some())
        .map(|(i, score)| (i, score.expect("Filtered out Nones already")))
        .collect();
    fuzz.sort_by_key(|(_i, score)| *score);
    let best_match_ingredient = fuzz.last().ok_or(anyhow!("Could not find ingredient: {ingredient_query}"))?.0;
    Ok(best_match_ingredient)
}

fn calc(state: State, all_recipes: RecipeMap, recipe: &str) -> Result<(), anyhow::Error> {
    let r = find_recipe(&all_recipes, recipe)?;
    r.print_calc(&state)?;
    Ok(())
}

fn mult(_state: State, all_recipes: RecipeMap, recipe: &str, ingredient: &str, amount: f64) -> Result<(), anyhow::Error> {
    let r = find_recipe(&all_recipes, recipe)?;

    let i = find_ingredient(r, ingredient)?;
    let factor = amount/i.quantity;

    println!("Standard Recipe: {}\n", r.name);
    println!("Out:");
    r.outputs().iter().for_each(|i| print_ingredient(i, None));
    println!("In:");
    r.inputs().iter().for_each(|i| print_ingredient(i, None));

    println!("\n{} [{} = {}] ({:.4})", r.name, i.part, amount, factor);
    println!("Out:");
    r.outputs().iter().for_each(|i| print_ingredient(i, Some(factor)));
    println!("In:");
    r.inputs().iter().for_each(|i| print_ingredient(i, Some(factor)));

    Ok(())
}

impl Recipe {
    pub fn print_calc(&self, state: &State) -> anyhow::Result<()> {
        let (max_belt, max_pipe) = self.max_outputs();
        let BlueprintCalc {
            use_belt,
            use_pipe,
            m_per_belt,
            m_per_pipe,
            n_boxes,
            pref_mult,
            clock,
            power_usage_mw,
        } = self.blueprint_calc(state)?;

        println!("\n{:12}{:>39}", self.building, self.name);
        println!("\n  --  IN  --");
        print_ingredient(&self.in_1, None);
        print_ingredient(&self.in_2, None);
        print_ingredient(&self.in_3, None);
        print_ingredient(&self.in_4, None);
        println!("\n  -- OUT  --");
        print_ingredient(&self.out_1, None);
        print_ingredient(&self.out_2, None);
        println!("\n  -- CALC --");

        if use_belt {
            println!("Max belt use: {:8}", max_belt);
        }
        if use_pipe {
            println!("Max pipe use: {:8}", max_pipe);
        }
        if use_belt {
            println!(
                "Num of {} per belt: {:8.4}",
                &self.building,
                m_per_belt,
            );
        }
        if use_pipe {
            println!(
                "Num of {} per pipe: {:8.4}",
                &self.building,
                m_per_pipe,
            );
        }

        let print_parts = |modifier: f64| {
            println!("Out:");
            print_ingredient(&self.out_1, Some(modifier));
            print_ingredient(&self.out_2, Some(modifier));
            println!("In:");
            print_ingredient(&self.in_1, Some(modifier));
            print_ingredient(&self.in_2, Some(modifier));
            print_ingredient(&self.in_3, Some(modifier));
            print_ingredient(&self.in_4, Some(modifier));
        };

        println!("\n  --  BP  --");
        println!("{} [{:.0}]", self.name, n_boxes);
        println!("Num {} per BP instance: {}", self.building, pref_mult);
        println!("Clock: {:5.2} %", clock * 100.0);
        println!("Power use: {:5.2} MW", power_usage_mw);
        print_parts(clock * n_boxes * pref_mult);
        if n_boxes > 1.0001 {
            println!("\n{:>34}", "Per BP Instance");
            print_parts(clock * pref_mult);
        }
        println!("\n{:>34}", format!("Per {}", self.building));
        print_parts(clock);

        Ok(())
    }

}

fn print_ingredient(i: &Option<Ingredient>, modify: Option<f64>) {
    let i = match i {
        Some(i) => i,
        None => return,
    };
    let t = match i.transport() {
        Transport::Belt => "Belt",
        Transport::Pipe => "Pipe",
    };
    match modify {
        None => println!("({:4})  {:27} {:15.4}", t, i.part, i.quantity),
        Some(m) => println!("  {:24} {:7.2}", i.part, m * i.quantity),
    }
}
