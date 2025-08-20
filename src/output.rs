use crate::chain::ChainState;
use crate::types::*;

impl Recipe {
    pub fn print_blueprint_suggestion(&self, state: &State) -> anyhow::Result<()> {
        let (max_belt, max_pipe) = self.max_outputs();
        let BlueprintSuggestion {
            use_belt,
            use_pipe,
            m_per_belt,
            m_per_pipe,
            n_boxes,
            pref_mult,
            clock,
            power_usage_mw,
        } = self.suggest_blueprint(state)?;

        println!("\n{:12}{:>39}", self.building, self.name);
        println!("\n  --  IN  --");
        self.inputs().for_each(|i| print_ingredient(i, None));
        println!("\n  -- OUT  --");
        self.outputs().for_each(|i| print_ingredient(i, None));
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
            self.outputs().for_each(|i| print_ingredient(i, Some(modifier)));
            println!("In:");
            self.inputs().for_each(|i| print_ingredient(i, Some(modifier)));
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

    pub fn print(&self) {
        println!("{}", self.name);
        println!("  Building: {}", self.building);
        println!("  Cycle time: {}", self.craft_time_s);
        println!("");
        println!("Out:");
        self.outputs().for_each(|i| print_ingredient(i, None));
        println!("In:");
        self.inputs().for_each(|i| print_ingredient(i, None));
    }
}

pub fn print_ingredient(i: &Ingredient, modify: Option<f64>) {
    match modify {
        None => println!("({:4})  {:27} {:15.4}", i.transport(), i.part, i.quantity),
        Some(m) => println!("  {:31} {:11.3}", i.part, m * i.quantity),
    }
}

// total width 64
pub fn print_chain(chain: &ChainState) {
    fn print_recipe_ingredient(i: &Ingredient, scale: f64, pmf: f64) {
        let q = i.quantity;
        let n = &i.part;
        let r = (q * pmf).round();
        let sq = format!("[{:.3}]", q * scale);
        println!(" {r:4} {n:31} {q:10.3} {sq:>15}");
    }
    for (_name, g) in chain.groups.iter() {
        let header = format!("---------- {} ----------", g.name);
        println!("{header:^64}");
        for (scale, r) in g.recipes.iter() {
            let n = &r.name;
            let b = &r.building;
            let s = format!("{scale:.3}");
            let bs = format!("{b} * {s}");
            println!("\n{n:31} {bs:>32}");

            let pmf = r.per_minute_factor();
            println!("Out:");
            r.outputs().for_each(|i| print_recipe_ingredient(i, *scale, pmf));
            println!("In:");
            r.inputs().for_each(|i| print_recipe_ingredient(i, *scale, pmf));
            println!("\n{:^64}", "_ _ _ _ _ _ _ _ _ _ _ _");
        }
        println!("\nINPUTS\n");
        for i in g.inputs.iter() {
            print_ingredient(i, Some(1.0));
        }
        println!("\nOUTPUTS\n");
        for i in g.outputs.iter() {
            print_ingredient(i, Some(1.0));
        }
        let b = g.balances();
        println!("\nABUNDANCE\n");
        for i in b.iter().filter(|i| i.quantity >= 0.0001) {
            print_ingredient(i, Some(1.0));
        }
        println!("\nPAUCITY\n");
        for i in b.iter().filter(|i| i.quantity < -0.0001) {
            print_ingredient(i, Some(-1.0));
        }

        // println!("{:#?}", b); // For debug
    }
}
