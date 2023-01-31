use plotters::backend::BitMapBackend;
use plotters::prelude::*;
use crate::mo::array_solution::{ArrayOptimizerParams, ArraySolution, ArraySolutionEvaluator, SolutionsRuntimeArrayProcessor};
use crate::mo::evaluator::DefaultEvaluator;
use crate::mo::optimizers::nsga2::NSGA2Optimizer;
use crate::mo::optimizers::Optimizer;
use crate::mo::Ratio;
use crate::mo::tests::dtlz::dtlz1::Dtlz1;

mod dtlz;

#[test]
fn optimizers_test() {

    let array_evaluators: Vec<Box<dyn ArraySolutionEvaluator>>= vec![ Box::new(Dtlz1::new(7, 3)) ];

    for array_solution_evaluator in array_evaluators.iter()
    {
        let optimizer_params = ArrayOptimizerParams::new(
            65,
            Ratio(1, 2),
            Ratio(3, 10),
            array_solution_evaluator.clone()
        );

        let mut optimizer: NSGA2Optimizer<ArraySolution> = NSGA2Optimizer::new(optimizer_params);

        optimizer.optimize(Box::new(DefaultEvaluator::new(100)),
                           Box::new(SolutionsRuntimeArrayProcessor{}));

        let best_solutions = optimizer.best_solutions();

        let root = BitMapBackend::new("S:/Downloads/test.png", (1920, 1080)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let mut chart = ChartBuilder::on(&root)
            .margin(20)
            .caption("DTLZ1 - NSGA-II", ("sans-serif", 40))
            .build_cartesian_3d(0.0..0.6, 0.0..0.6, 0.0..0.6)
            .unwrap();
        chart.configure_axes().draw().unwrap();

        let mut points = vec![];

        for solution in best_solutions
        {
            points.push((solution.1.f[0], solution.1.f[1], solution.1.f[2]));
        }

        chart.draw_series(PointSeries::of_element(
            points,
            5,
            &RED,
            &|c, s, st| {
                return EmptyElement::at(c)    // We want to construct a composed element on-the-fly
                    + Circle::new((0,0),s,st.filled()) // At this point, the new pixel coordinate is established
                    + Text::new(format!(""), (10, 0), ("sans-serif", 10).into_font());
            },
        )).unwrap();

    }
}
