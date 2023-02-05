use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::iter::{Map, Zip};
use std::slice::Iter;
use itertools::Itertools;
use markdown_table::MarkdownTable;
use plotters::backend::BitMapBackend;
use plotters::prelude::*;
use crate::mo::array_solution::{ArrayOptimizerParams, ArraySolution, ArraySolutionEvaluator, SolutionsRuntimeArrayProcessor};
use crate::mo::evaluator::DefaultEvaluator;
use crate::mo::optimizers::nsga2::NSGA2Optimizer;
use crate::mo::optimizers::Optimizer;
use crate::mo::problem::Problem;
use crate::mo::Ratio;
use crate::mo::tests::dtlz::dtlz1::Dtlz1;
use std::io::Write;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use rand::{Rng, thread_rng};
use crate::mo::tests::dtlz::dtlz2::Dtlz2;
use crate::mo::tests::dtlz::dtlz3::Dtlz3;
use crate::mo::tests::dtlz::dtlz4::Dtlz4;
use crate::mo::tests::dtlz::dtlz5::Dtlz5;
use crate::mo::tests::dtlz::dtlz6::Dtlz6;
use crate::mo::tests::dtlz::dtlz7::Dtlz7;

mod dtlz;

fn optimize_and_get_best_solutions(optimizer: &mut Box<dyn Optimizer<ArraySolution>>, terminate_early_count: usize) -> Vec<(Vec<f64>, ArraySolution)>
{
    optimizer.optimize(Box::new(DefaultEvaluator::new(terminate_early_count)),
                       Box::new(SolutionsRuntimeArrayProcessor{}));

    optimizer.best_solutions()
}

fn mean_convergence_metric_for_solutions(problem: &Box<dyn Problem + Send>, solutions: &Vec<(Vec<f64>, ArraySolution)>) -> f64
{
    let sum = solutions
        .iter()
        .map(|solution| problem.convergence_metric(&solution.1.x))
        .sum::<f64>();

    sum / solutions.len() as f64
}

fn print_best_solutions_3d_to_gif(problem: &Box<dyn Problem + Send>,
                                  optimizer: &Box<dyn Optimizer<ArraySolution>>,
                                  best_solutions: &Vec<(Vec<f64>, ArraySolution)>,
                                  path: &std::path::Path)
{
    let root = BitMapBackend::gif(path, (1920, 1080), 100).unwrap().into_drawing_area();

    for pitch in 0..157 {
        root.fill(&WHITE).unwrap();
        let mut chart = ChartBuilder::on(&root)
            .margin(20)
            .caption(format!("{} - {} [{:.2}]", problem.name(), optimizer.name(), mean_convergence_metric_for_solutions(problem, best_solutions)), ("sans-serif", 40))
            .build_cartesian_3d(problem.plot_3d_min_x()..problem.plot_3d_max_x(),
                                problem.plot_3d_min_y()..problem.plot_3d_max_y(),
                                problem.plot_3d_min_z()..problem.plot_3d_max_z())
            .unwrap();

        chart.with_projection(|mut p| {
            p.pitch = 1.57 - (1.57 - pitch as f64 / 50.0).abs();
            p.scale = 0.7;
            p.into_matrix() // build the projection matrix
        });

        chart.configure_axes().draw().unwrap();

        chart.draw_series(PointSeries::of_element(
            best_solutions.iter()
                .map(|solution|
                    (solution.1.f[0], solution.1.f[1], solution.1.f[2])
                ),
            5,
            &RED,
            &|c, s, st| {
                return EmptyElement::at(c)    // We want to construct a composed element on-the-fly
                    + Circle::new((0,0),s,st.filled()) // At this point, the new pixel coordinate is established
                    + Text::new(format!(""), (10, 0), ("sans-serif", 10).into_font());
            },
        )).unwrap();

        root.present().unwrap();
    }

    root.present().unwrap();
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
    test_problems: Vec<(Box<dyn ArraySolutionEvaluator + Send>, Box<dyn Problem + Send>)>,
    optimizer_creators: Vec<fn(ArrayOptimizerParams) -> Box<dyn Optimizer<ArraySolution>>>
}

impl ProblemsSolver
{
    pub fn new(test_problems: Vec<(Box<dyn ArraySolutionEvaluator + Send>, Box<dyn Problem + Send>)>,
               optimizer_creators: Vec<fn(ArrayOptimizerParams) -> Box<dyn Optimizer<ArraySolution>>>) -> Self
    {
        ProblemsSolver {
            test_problems,
            optimizer_creators
        }
    }

    fn create_test_problem<T: ArraySolutionEvaluator + Send + Problem + Clone + 'static>(problem: &T) -> (Box<dyn ArraySolutionEvaluator + Send>, Box<dyn Problem + Send>)
    {
        (Box::new(problem.clone()), Box::new(problem.clone()))
    }

    fn calc_best_solutions_and_print_to_3d_plots(&self, dir: &std::path::Path)
    {
        let mut multi_threaded_runtime = tokio::runtime::Builder::new_multi_thread().build().unwrap();

        multi_threaded_runtime.block_on(async move {

            let mut tasks = vec![];

            self.optimizer_creators
                .iter()
                .cartesian_product(&self.test_problems)
                .for_each(|problematic| {

                    let array_solution_evaluator = problematic.1.0.clone();
                    let problem = problematic.1.1.clone();
                    let optimizer_creator = problematic.0.clone();
                    let dir = String::from(dir.to_str().unwrap());

                    tasks.push(tokio::spawn(async move {
                        let dir = std::path::Path::new(&dir);

                        let array_optimizer_params = new_array_optimizer_params(array_solution_evaluator);

                        let mut optimizer = optimizer_creator(array_optimizer_params);

                        println!("Optimizing {} - {}", optimizer.name(), problem.name());

                        let best_solutions = optimize_and_get_best_solutions(&mut optimizer, 1000);

                        print_best_solutions_3d_to_gif(&problem,
                                                       &optimizer,
                                                       &best_solutions,
                                                       &dir.join(format!("{} - {}.gif", optimizer.name(), problem.name())));
                    }));


                });

            for task in tasks
            {
                task.await.unwrap();
            }
        });
    }

    fn calc_metric_and_save_to_file(&self, repeat_count: usize, dir: &std::path::Path)
    {
        let mut optimizer_names: Arc<tokio::sync::Mutex<HashSet<String>>> = Arc::new(tokio::sync::Mutex::new(HashSet::new()));

        let mut table_lines = Vec::new();

        let mut multi_threaded_runtime = tokio::runtime::Builder::new_multi_thread().build().unwrap();

        let optimizer_names_task = optimizer_names.clone();
        multi_threaded_runtime.block_on(async move {
            let mut tasks = vec![];

            for test_problem in &self.test_problems
            {
                let mut problems_results_table = vec!["".to_string(); self.optimizer_creators.len() + 1];

                problems_results_table[0] = test_problem.1.name().to_string();

                let test_problem_index = table_lines.len();

                for (optimizer_index, optimizer_creator) in self.optimizer_creators.iter().enumerate()
                {
                    let array_solution_evaluator = test_problem.0.clone();
                    let problem = test_problem.1.clone();
                    let optimizer_creator = (*optimizer_creator).clone();

                    let optimizer_names = optimizer_names_task.clone();
                    tasks.push(tokio::spawn(async move {
                        let mut tasks = vec![];

                        for _ in 0..repeat_count
                        {
                            let array_solution_evaluator = array_solution_evaluator.clone();
                            let problem = problem.clone();

                            let optimizer_creator = optimizer_creator.clone();
                            let optimizer_names = optimizer_names.clone();
                            tasks.push(tokio::spawn(async move {
                                let (optimizer_name, metric) =
                                    {
                                        let array_optimizer_params = new_array_optimizer_params(array_solution_evaluator);

                                        let mut optimizer = optimizer_creator(array_optimizer_params);

                                        println!("Optimizing {} - {}", optimizer.name(), problem.name());

                                        let best_solutions = optimize_and_get_best_solutions(&mut optimizer, 1000);

                                        let optimizer_name = optimizer.name().to_string();

                                        let metric = mean_convergence_metric_for_solutions(&problem, &best_solutions);

                                        (optimizer_name, metric)
                                    };

                                {
                                    optimizer_names.lock().await.insert(optimizer_name);
                                }

                                metric
                            }));
                        }

                        let mut summ_metric = 0.0;

                        for task in tasks
                        {
                            summ_metric += task.await.unwrap();
                        }

                        (optimizer_index + 1, test_problem_index, summ_metric / repeat_count as f64)
                    }));
                }

                table_lines.push(problems_results_table);
            }

            for task in tasks
            {
                let result = task.await.unwrap();

                table_lines[result.1][result.0] = format!("{:.2}", result.2);
            }

            table_lines.sort_by(|a, b| a[0].cmp(&b[0]));

            let mut optimizers_title = vec!["".to_string()];

            for optimizer_name in optimizer_names.lock().await.iter()
            {
                optimizers_title.push(optimizer_name.clone());
            }

            table_lines.insert(0, optimizers_title);

            let table = MarkdownTable::new(table_lines);

            println!("{}", table.to_string());

            let mut output = File::create(dir.join("metric.html")).unwrap();
            write!(output, "{}", table.to_string()).unwrap();
        });
    }
}

fn dtlz_test_problems(n_var: usize, n_obj: usize) -> Vec<(Box<dyn ArraySolutionEvaluator + Send>, Box<dyn Problem + Send>)> {
    let mut test_problems = vec![];

    test_problems.push(ProblemsSolver::create_test_problem(&Dtlz1::new(n_var, n_obj)));
    test_problems.push(ProblemsSolver::create_test_problem(&Dtlz2::new(n_var, n_obj)));
    test_problems.push(ProblemsSolver::create_test_problem(&Dtlz3::new(n_var, n_obj)));
    test_problems.push(ProblemsSolver::create_test_problem(&Dtlz4::new(n_var, n_obj)));
    test_problems.push(ProblemsSolver::create_test_problem(&Dtlz5::new(n_var, n_obj)));
    test_problems.push(ProblemsSolver::create_test_problem(&Dtlz6::new(n_var, n_obj)));
    test_problems.push(ProblemsSolver::create_test_problem(&Dtlz7::new(n_var, n_obj)));

    test_problems
}

fn calc_std_dev_for_problem(problem: &Box<dyn Problem + Send>, evaluator: &Box<dyn ArraySolutionEvaluator + Send>) -> f64
{
    let mut x = vec![0.0; evaluator.x_len()];

    let mut rng = thread_rng();

    let mut sum = 0.0;

    let count = 10_000_000;

    let mut metrics = vec![];

    for _ in 0..count
    {
        for x_i in x.iter_mut()
        {
            *x_i = rng.gen_range(evaluator.min_x_value()..=evaluator.max_x_value());
        }

        let metric = problem.convergence_metric(&x);

        sum += metric;

        metrics.push(metric);
    }

    let mean = sum / count as f64;

    let mut sum_mean = 0.0;
    for metric in metrics
    {
        sum_mean += (metric - 0.0).powi(2);
    }

    (sum_mean / count as f64).sqrt()
}

#[test]
fn print_std_dev_for_metrics()
{
    let test_problem = ProblemsSolver::create_test_problem(&Dtlz1::new(4, 3));

    println!("{}", calc_std_dev_for_problem(&test_problem.1, &test_problem.0));
}

#[test]
fn print_3d_images_for_optimizers() {
    let mut test_problems = vec![];

    for n_var in vec![4, 7, 15, 20, 30]
    {
        test_problems.extend(dtlz_test_problems(n_var, 3));
    }

    let problem_solver = ProblemsSolver::new(
        test_problems,
        vec![
            |optimizer_params: ArrayOptimizerParams| Box::new(NSGA2Optimizer::new(optimizer_params))
        ]
    );

    problem_solver.calc_best_solutions_and_print_to_3d_plots(std::path::Path::new("D:/tmp/test_optimizers"));
}

#[test]
fn calc_output_metric_for_optimizers() {

    let mut test_problems = vec![];

    for n_var in vec![4, 7, 15, 20, 30]
    {
        for n_obj in vec![3, 5, 10, 15, 25]
        {
            if n_obj >= n_var
            {
                continue
            }

            test_problems.extend(dtlz_test_problems(n_var, n_obj));
        }
    }

    let problem_solver = ProblemsSolver::new(
        test_problems,
        vec![
            |optimizer_params: ArrayOptimizerParams| Box::new(NSGA2Optimizer::new(optimizer_params))
        ]
    );

    problem_solver.calc_metric_and_save_to_file(10, std::path::Path::new("D:/tmp/test_optimizers"));
}
