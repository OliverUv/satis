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
    let mut key = None;
    for k in all_recipes.keys() {
        let kl = k.as_str().to_lowercase();
        let search_key = recipe.to_lowercase();
        if kl == search_key { key = Some(k.clone()); }
    }
    let k = key.ok_or(anyhow!("No such recipe: {}", &recipe))?;
    let r = all_recipes.get(&k).ok_or(anyhow!("Could not find recipe: {k}"))?;
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

        let in_out_modifier = clock * n_boxes * pref_mult;

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

        println!("\n  --  BP  --");
        println!("{} [{:.0}]", self.name, n_boxes);
        println!("Num {} per instance: {}", self.building, pref_mult);
        println!("Clock: {:5.2} %", clock * 100.0);
        println!("Out:");
        print_ingredient(&self.out_1, Some(in_out_modifier));
        print_ingredient(&self.out_2, Some(in_out_modifier));
        println!("In:");
        print_ingredient(&self.in_1, Some(in_out_modifier));
        print_ingredient(&self.in_2, Some(in_out_modifier));
        print_ingredient(&self.in_3, Some(in_out_modifier));
        print_ingredient(&self.in_4, Some(in_out_modifier));

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

