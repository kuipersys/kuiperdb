use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use kuiperdb_core::{Database, IndexConfig, RecordInput, RelationInput, VectorQuery, VectorSpace};
use serde::Deserialize;

#[derive(Clone)]
pub struct AppState {
    pub database: Database,
}

fn internal(error: anyhow::Error) -> actix_web::Error {
    actix_web::error::ErrorInternalServerError(error.to_string())
}

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

#[post("/spaces")]
async fn create_space(
    state: web::Data<AppState>,
    body: web::Json<VectorSpace>,
) -> actix_web::Result<impl Responder> {
    state
        .database
        .create_vector_space(body.into_inner())
        .await
        .map_err(internal)?;
    Ok(HttpResponse::Created().finish())
}

#[get("/spaces")]
async fn list_spaces(state: web::Data<AppState>) -> actix_web::Result<impl Responder> {
    Ok(web::Json(
        state
            .database
            .list_vector_spaces()
            .await
            .map_err(internal)?,
    ))
}

#[delete("/spaces/{name}")]
async fn delete_space(
    state: web::Data<AppState>,
    name: web::Path<String>,
) -> actix_web::Result<impl Responder> {
    if state
        .database
        .delete_vector_space(&name)
        .await
        .map_err(internal)?
    {
        Ok(HttpResponse::NoContent().finish())
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}

#[post("/records")]
async fn put_record(
    state: web::Data<AppState>,
    body: web::Json<RecordInput>,
) -> actix_web::Result<impl Responder> {
    Ok(HttpResponse::Created().json(
        state
            .database
            .put_record(body.into_inner())
            .await
            .map_err(internal)?,
    ))
}

#[post("/records/bulk")]
async fn put_records(
    state: web::Data<AppState>,
    body: web::Json<Vec<RecordInput>>,
) -> actix_web::Result<impl Responder> {
    Ok(HttpResponse::Created().json(
        state
            .database
            .put_records(body.into_inner())
            .await
            .map_err(internal)?,
    ))
}

#[get("/records/{id}")]
async fn get_record(
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> actix_web::Result<impl Responder> {
    match state.database.get_record(&id).await.map_err(internal)? {
        Some(record) => Ok(HttpResponse::Ok().json(record)),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

#[derive(Deserialize)]
struct Page {
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
}
fn default_limit() -> usize {
    100
}

#[get("/records")]
async fn list_records(
    state: web::Data<AppState>,
    page: web::Query<Page>,
) -> actix_web::Result<impl Responder> {
    Ok(web::Json(
        state
            .database
            .list_records(page.limit.min(10_000), page.offset)
            .await
            .map_err(internal)?,
    ))
}

#[delete("/records/{id}")]
async fn delete_record(
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> actix_web::Result<impl Responder> {
    if state.database.delete_record(&id).await.map_err(internal)? {
        Ok(HttpResponse::NoContent().finish())
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}

#[post("/search")]
async fn search(
    state: web::Data<AppState>,
    body: web::Json<VectorQuery>,
) -> actix_web::Result<impl Responder> {
    Ok(web::Json(
        state
            .database
            .search(body.into_inner())
            .await
            .map_err(internal)?,
    ))
}

#[post("/relations")]
async fn create_relation(
    state: web::Data<AppState>,
    body: web::Json<RelationInput>,
) -> actix_web::Result<impl Responder> {
    Ok(HttpResponse::Created().json(
        state
            .database
            .create_relation(body.into_inner())
            .await
            .map_err(internal)?,
    ))
}

#[get("/records/{id}/relations")]
async fn relations_for(
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> actix_web::Result<impl Responder> {
    Ok(web::Json(
        state.database.relations_for(&id).await.map_err(internal)?,
    ))
}

#[delete("/relations/{id}")]
async fn delete_relation(
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> actix_web::Result<impl Responder> {
    if state
        .database
        .delete_relation(&id)
        .await
        .map_err(internal)?
    {
        Ok(HttpResponse::NoContent().finish())
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}

#[post("/spaces/{name}/index/rebuild")]
async fn rebuild_index(
    state: web::Data<AppState>,
    name: web::Path<String>,
) -> actix_web::Result<impl Responder> {
    Ok(web::Json(
        state
            .database
            .rebuild_index(&name)
            .await
            .map_err(internal)?,
    ))
}

#[get("/spaces/{name}/index")]
async fn index_status(
    state: web::Data<AppState>,
    name: web::Path<String>,
) -> actix_web::Result<impl Responder> {
    Ok(web::Json(
        state.database.index_status(&name).await.map_err(internal)?,
    ))
}

#[put("/spaces/{name}/index")]
async fn update_index_config(
    state: web::Data<AppState>,
    name: web::Path<String>,
    body: web::Json<IndexConfig>,
) -> actix_web::Result<impl Responder> {
    state
        .database
        .update_index_config(&name, body.into_inner())
        .await
        .map_err(internal)?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(health)
        .service(create_space)
        .service(list_spaces)
        .service(delete_space)
        .service(put_record)
        .service(put_records)
        .service(get_record)
        .service(list_records)
        .service(delete_record)
        .service(search)
        .service(create_relation)
        .service(relations_for)
        .service(delete_relation)
        .service(rebuild_index)
        .service(index_status)
        .service(update_index_config);
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use kuiperdb_core::SearchResult;

    #[actix_web::test]
    async fn vector_first_http_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let database = Database::open(directory.path().join("api.db"))
            .await
            .unwrap();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState { database }))
                .configure(configure),
        )
        .await;

        let request = test::TestRequest::post().uri("/spaces")
            .set_json(serde_json::json!({"name":"semantic","dimensions":3,"distance_metric":"cosine","normalization":"unit"}))
            .to_request();
        assert_eq!(test::call_service(&app, request).await.status(), 201);

        let request = test::TestRequest::post().uri("/records")
            .set_json(serde_json::json!({"id":"stable","payload":{"kind":"example"},"vectors":[{"space":"semantic","values":[1.0,0.0,0.0]}]}))
            .to_request();
        assert_eq!(test::call_service(&app, request).await.status(), 201);

        let request = test::TestRequest::post()
            .uri("/search")
            .set_json(serde_json::json!({"space":"semantic","vector":[1.0,0.0,0.0],"limit":1}))
            .to_request();
        let results: Vec<SearchResult> = test::call_and_read_body_json(&app, request).await;
        assert_eq!(results[0].record.id, "stable");
    }
}
