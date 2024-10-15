use std::collections::HashMap;

/// Imports recipe data.
/// Uses the google sheets data from this project:
/// https://steamcommunity.com/sharedfiles/filedetails/?id=2874178191
/// (go to Production Recipes tab, then export as csv)

use anyhow;

pub mod types;
use types::*;

fn main() -> Result<(), anyhow::Error> {
    let stdin = std::io::stdin();
    {
        // Skip two lines, as they are headers which we won't use
        let mut input = String::new();
        stdin.read_line(&mut input)?;
        stdin.read_line(&mut input)?;
    }
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(stdin);
    let mut recipes = Vec::new();
    for res in reader.records() {
        let record = res?;
        let recipe = parse_recipe(&record);
        match recipe {
            Ok(Some(r)) => recipes.push(r),
            Ok(None) => continue,
            Err(e) => {
                println!("Error at record {:?}", record);
                return Err(e);
            }
        }
    }

    // for r in recipes.iter() {
    //     println!("{:?}", r);
    // }

    let mut rmap = Recipes { recipes: HashMap::new() };
    for r in recipes.into_iter() {
        let name = r.name.clone();
        rmap.recipes.insert(name, r);
    }
    let toml = toml::to_string_pretty(&rmap)?;
    println!("{toml}");

    Ok(())
}

fn parse_recipe(record: &csv::StringRecord) -> Result<Option<Recipe>, anyhow::Error> {
    let fields: Vec<&str> = record.iter().collect();
    if fields[0].is_empty() { return Ok(None); }
    Ok(Some(Recipe {
        building: fields[0].into(),
        name: fields[1].into(),
        craft_time: fields[2].parse()?,
        is_alt: fields[3] == "TRUE",
        unlocks: fields[4].into(),
        is_unlocked: fields[5] == "TRUE",
        in_1: parse_ingredient(fields[6], fields[7])?,
        in_2: parse_ingredient(fields[8], fields[9])?,
        in_3: parse_ingredient(fields[10], fields[11])?,
        in_4: parse_ingredient(fields[12], fields[13])?,
        out_1: parse_ingredient(fields[14], fields[15])?,
        out_2: parse_ingredient(fields[16], fields[17])?,
    }))
}

fn parse_ingredient(part: &str, quantity: &str) -> Result<Option<Ingredient>, anyhow::Error> {
    if part.is_empty() || quantity.is_empty() {
        return Ok(None);
    }

    Ok(Some(Ingredient{
        part: part.into(),
        quantity: quantity.parse()?
    }))
}
