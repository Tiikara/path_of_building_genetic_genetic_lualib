use std::iter::{Map, Zip};
use std::slice::Iter;
use itertools::Itertools;
use plotters::backend::BitMapBackend;
use plotters::prelude::*;
use crate::mo::array_solution::{ArrayOptimizerParams, ArraySolution, ArraySolutionEvaluator, SolutionsRuntimeArrayProcessor};
use crate::mo::evaluator::DefaultEvaluator;
use crate::mo::optimizers::nsga2::NSGA2Optimizer;
use crate::mo::optimizers::Optimizer;
use crate::mo::problem::Problem;
use crate::mo::Ratio;
use crate::mo::tests::dtlz::dtlz1::Dtlz1;

mod dtlz;

fn optimize_and_get_best_solutions(optimizer: &mut Box<dyn Optimizer<ArraySolution>>, terminate_early_count: usize) -> Vec<(Vec<f64>, ArraySolution)>
{
    optimizer.optimize(Box::new(DefaultEvaluator::new(terminate_early_count)),
                       Box::new(SolutionsRuntimeArrayProcessor{}));

    optimizer.best_solutions()
}

fn mean_convergence_metric_for_solutions(problem: &Box<dyn Problem>, solutions: &Vec<(Vec<f64>, ArraySolution)>) -> f64
{
    let sum = solutions
        .iter()
        .map(|solution| problem.convergence_metric(&solution.1.x))
        .sum::<f64>();

    sum / solutions.len() as f64
}

fn print_best_solutions_3d_to_image(problem: &Box<dyn Problem>,
                                    optimizer: &Box<dyn Optimizer<ArraySolution>>,
                                    best_solutions: &Vec<(Vec<f64>, ArraySolution)>,
                                    path: &std::path::Path)
{
    let root = BitMapBackend::new(path, (1920, 1080)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let mut chart = ChartBuilder::on(&root)
        .margin(20)
        .caption(format!("{} - {} [{:.2}]", problem.name(), optimizer.name(), problem.convergence_metric(&best_solutions.first().unwrap().1.x)), ("sans-serif", 40))
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

fn new_array_optimizer_params(array_solution_evaluator: Box<dyn ArraySolutionEvaluator>) -> ArrayOptimizerParams
{
    ArrayOptimizerParams::new(
        65,
        Ratio(1, 2),
        Ratio(3, 10),
        array_solution_evaluator
    )
}

struct ProblemsSolver
{
    test_problems: Vec<(Box<dyn ArraySolutionEvaluator>, Box<dyn Problem>)>,
    optimizer_creators: Vec<Box<dyn Fn(ArrayOptimizerParams) -> Box<dyn Optimizer<ArraySolution>>>>
}

impl ProblemsSolver
{
    pub fn new(test_problems: Vec<(Box<dyn ArraySolutionEvaluator>, Box<dyn Problem>)>, optimizer_creators: Vec<Box<dyn Fn(ArrayOptimizerParams) -> Box<dyn Optimizer<ArraySolution>>>>) -> Self
    {
        ProblemsSolver {
            test_problems,
            optimizer_creators
        }
    }

    fn create_test_problem<T: ArraySolutionEvaluator + Problem + Clone + 'static>(problem: &T) -> (Box<dyn ArraySolutionEvaluator>, Box<dyn Problem>)
    {
        (Box::new(problem.clone()), Box::new(problem.clone()))
    }

    fn iter_optimizer_problem_best_solution(&self) -> impl Iterator<Item = (Box<dyn Optimizer<ArraySolution>>, &(Box<dyn ArraySolutionEvaluator>, Box<dyn Problem>), Vec<(Vec<f64>, ArraySolution)>)>
    {
        self.optimizer_creators
            .iter()
            .cartesian_product(&self.test_problems)
            .map(|problematic| {
                let array_optimizer_params = new_array_optimizer_params(problematic.1.0.clone());

                let mut optimizer = problematic.0(array_optimizer_params);

                println!("Optimizing {} - {}", optimizer.name(), problematic.1.1.name());

                let best_solutions = optimize_and_get_best_solutions(&mut optimizer, 10);

                (optimizer, problematic.1, best_solutions)
            })
    }

    fn calc_best_solutions_and_print_to_3d_plots(&self, dir: &std::path::Path)
    {
        for iter_item in self.iter_optimizer_problem_best_solution()
        {
            let test_problem = iter_item.1;
            let optimizer = iter_item.0;
            let best_solutions = iter_item.2;

            print_best_solutions_3d_to_image(&test_problem.1,
                                             &optimizer,
                                             &best_solutions,
                                             &dir.join(format!("{} - {}.png", optimizer.name(), test_problem.1.name())));
        }
    }

    fn calc_metric(&self, repeat_count: usize, dir: &std::path::Path)
    {
        self.optimizer_creators
            .iter()
            .cartesian_product(&self.test_problems)
            .for_each(|problematic| {
                let mut summ_metric = 0.0;

                for _ in 0..repeat_count
                {
                    let array_optimizer_params = new_array_optimizer_params(problematic.1.0.clone());

                    let mut optimizer = problematic.0(array_optimizer_params);

                    println!("Optimizing {} - {}", optimizer.name(), problematic.1.1.name());

                    let best_solutions = optimize_and_get_best_solutions(&mut optimizer, 10);

                    summ_metric += mean_convergence_metric_for_solutions(&problematic.1.1, &best_solutions);
                }

                println!("{}", summ_metric / repeat_count as f64);
            });
    }
}

#[test]
fn print_3d_images_for_optimizers() {
    let problem_solver = ProblemsSolver::new(
        vec![
            ProblemsSolver::create_test_problem(&Dtlz1::new(4, 3)),
            ProblemsSolver::create_test_problem(&Dtlz1::new(7, 3)),
            ProblemsSolver::create_test_problem(&Dtlz1::new(15, 3)),
            ProblemsSolver::create_test_problem(&Dtlz1::new(20, 3)),
            ProblemsSolver::create_test_problem(&Dtlz1::new(30, 3))
        ],
        vec![
            Box::new(|optimizer_params: ArrayOptimizerParams| Box::new(NSGA2Optimizer::new(optimizer_params)))
        ]
    );

    problem_solver.calc_best_solutions_and_print_to_3d_plots(std::path::Path::new("D:/tmp/test_optimizers"));
}

#[test]
fn calc_output_metric_for_optimizers() {

    let mut test_problems = vec![];

    for n_var in vec![4, 7, 15, 20]
    {
        for n_obj in vec![3, 5, 10, 15]
        {
            if n_obj >= n_var
            {
                continue
            }

            test_problems.push(ProblemsSolver::create_test_problem(&Dtlz1::new(n_var, n_obj)));
        }
    }

    let problem_solver = ProblemsSolver::new(
        test_problems,
        vec![
            Box::new(|optimizer_params: ArrayOptimizerParams| Box::new(NSGA2Optimizer::new(optimizer_params)))
        ]
    );

    problem_solver.calc_metric(1, std::path::Path::new("D:/tmp/test_optimizers"));
}
