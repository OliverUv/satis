use crate::types::*;

pub fn process_chain(_state: State, all_recipes: RecipeMap, chain: Vec<String>) -> Result<(), anyhow::Error> {
    for l in chain.iter() {
        println!("{}", l);
    }
    todo!()
}
