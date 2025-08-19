//! Imports recipe data.
//! Uses the google sheets data from this project:
//! https://steamcommunity.com/sharedfiles/filedetails/?id=2874178191
//! (go to Production Recipes tab, then export as csv)
//!
//! You MUST remove the first two header lines from the csv file.
//! You SHOULD remove all the weird ,,,FALSE,,, lines at the bottom of the file.

use anyhow::Result;

use crate::types::*;

pub fn recipe_file() -> &'static str {
    include_str!("../recipes.csv")
}

pub fn get_all_recipes() -> Result<RecipeCollection> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(recipe_file().as_bytes());
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

    apply_patches(&mut recipes)?;
    add_custom(&mut recipes)?;

    Ok(recipes)
}

fn apply_patches(recipes: &mut RecipeCollection) -> Result<()> {
    // Diamonds -> Time Crystals, should be 10s 2:1 ratio
    let err = || anyhow::anyhow!("Could not apply patch to Time Crystal recipe");
    let tc = recipe_by_name_mut(recipes, "Time Crystal").ok_or_else(err)?;
    let diamonds = tc.in_1.as_mut().ok_or_else(err)?;
    diamonds.quantity = 12.0;
    Ok(())
}

fn add_custom(recipes: &mut RecipeCollection) -> Result<()> {
    recipes.push(Recipe {
        building: "Nuclear Power Plant".into(),
        name: "Burn Uranium".into(),
        craft_time_s: 300.,
        is_alt: false,
        unlocks: "".to_string(),
        is_unlocked: true,
        in_1: Some(Ingredient {
            part: "Uranium Fuel Rod".into(),
            quantity: 0.2,
        }),
        in_2: Some(Ingredient {
            part: "Water".into(),
            quantity: 240.,
        }),
        in_3: None,
        in_4: None,
        out_1: Some(Ingredient {
            part: "Uranium Waste".into(),
            quantity: 10.,
        }),
        out_2: None,
    });
    Ok(())
}

fn parse_recipe(record: &csv::StringRecord) -> Result<Option<Recipe>> {
    let fields: Vec<&str> = record.iter().collect();
    if fields[0].is_empty() { return Ok(None); }
    Ok(Some(Recipe {
        building: fields[0].into(),
        name: fields[1].into(),
        craft_time_s: fields[2].parse()?,
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

fn parse_ingredient(part: &str, quantity: &str) -> Result<Option<Ingredient>> {
    if part.is_empty() || quantity.is_empty() {
        return Ok(None);
    }

    Ok(Some(Ingredient{
        part: part.into(),
        quantity: quantity.parse()?
    }))
}
