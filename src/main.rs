use anyhow::anyhow;
use clap::{Parser, Subcommand};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

pub mod types;
use types::*;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Calc{recipe: String},
    // List,
}

fn main() -> Result<(), anyhow::Error> {
    // println!("Reading recipes");
    let all_recipes = std::fs::read_to_string("./all_recipes.toml")?;
    let all_recipes = toml::from_str::<Recipes>(&all_recipes)?.recipes;

    let state = State::default();

    let cli = Cli::parse();
    match &cli.command {
        Command::Calc{recipe} => calc(state, all_recipes, recipe.as_str())?,
    }

    Ok(())
}

fn calc(state: State, all_recipes: RecipeMap, recipe: &str) -> Result<(), anyhow::Error> {
    let matcher = SkimMatcherV2::default();
    let mut fuzz: Vec<(&str, i64)> = all_recipes.keys()
        .map(String::as_str)
        .map(|key| (key, matcher.fuzzy_match(key, recipe)))
        .filter(|(_key, score)| score.is_some())
        .map(|(key, score)| (key, score.expect("Filtered out Nones already")))
        .collect();
    fuzz.sort_by_key(|(_key, score)| *score);
    let best_match_key = fuzz.last().ok_or(anyhow!("Could not find recipe: {recipe}"))?.0;
    let r = all_recipes.get(best_match_key).ok_or(anyhow!("Could not find recipe: {best_match_key}"))?;
    r.print_calc(&state)?;
    Ok(())
}

impl Recipe {
    pub fn print_calc(&self, state: &State) -> anyhow::Result<()> {
        let (max_belt, max_pipe) = self.max_outputs();
        let RecipeCalc {
            use_belt,
            use_pipe,
            m_per_belt,
            m_per_pipe,
            n_boxes,
            pref_mult,
            clock,
        } = self.calc(state)?;

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

