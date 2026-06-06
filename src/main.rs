// See src/lib.rs: raised so cargo-leptos release builds don't hit E0275
// (trait-solver overflow) on nested leptos/tachys view tuples. No behavior change.
#![recursion_limit = "256"]

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use icebarn_rs::app::*;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use sqlx::postgres::PgPoolOptions;

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    let pg_uri = std::env::var("POSTGRES_URI").unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(50)
        .connect(&pg_uri)
        .await
        .unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rooms (pwd TEXT, ts TIMESTAMPTZ DEFAULT now(), state BYTEA)",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query("CREATE INDEX IF NOT EXISTS rooms_pwd_ts ON rooms(pwd, ts)")
        .execute(&pool)
        .await
        .unwrap();

    let app = Router::new()
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            move || provide_context(pool.clone()),
            {
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
