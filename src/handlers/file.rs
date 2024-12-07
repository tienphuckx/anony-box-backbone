use crate::{
  errors::{ApiError, DBError},
  extractors::UserToken,
  payloads::minors::FileResponse,
  utils::minors::{generate_file_name_with_timestamp, get_server_url, guess_mime_type_from_path},
  AppState, UPLOADS_DIRECTORY,
};
use axum::{
  body::{Body, Bytes},
  extract::{Path, State},
  http::{header, StatusCode},
  response::{IntoResponse, Response},
  BoxError, Json,
};
use axum_extra::extract::Multipart;
use futures::{Stream, TryFutureExt, TryStreamExt};
use std::{io, path::PathBuf, sync::Arc};
use tokio::{
  fs::File,
  io::{BufReader, BufWriter},
};
use tokio_util::io::{ReaderStream, StreamReader};
use utoipa::ToSchema;

///### Handler to serve static files efficiently with streaming
#[utoipa::path(
  get,
  path = "/files/{filename}",
  params(
    ("filename" = String, Path, description = "name of file"),
  ),
  responses(
      (status = 200, description = "OK")
  )
)]
pub async fn serve_file(Path(filename): Path<String>) -> Response {
  // Construct the path to the static file directory
  let base_path = PathBuf::from(UPLOADS_DIRECTORY);
  let file_path = base_path.join(filename);

  // Open the file in streaming mode
  match File::open(&file_path).await {
    Ok(file) => {
      let stream: ReaderStream<BufReader<File>> = ReaderStream::new(BufReader::new(file));
      let body = Body::from_stream(stream);

      // Determine the content type
      let content_type = guess_mime_type_from_path(file_path);

      // Build and return the response
      Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .body(body)
        .unwrap()
    }
    Err(_) => {
      // Return a 404 response if the file doesn't exist
      (StatusCode::NOT_FOUND, "404: File not found".to_string()).into_response()
    }
  }
}

#[allow(dead_code)]
#[derive(ToSchema, Debug)]
pub struct UploadFile {
  #[schema(value_type = String, format = Binary)]
  pub file: Vec<u8>,
}

/// ### Handler to upload a file to server
#[utoipa::path(
    post,
    params(
      (
        "x-user-code" = String, Header, description = "user code for authentication",
        example = "6C70F6E0A888C1360AD532C66D8F1CD0ED48C1CC47FA1AE6665B1FC3DAABB468"
      ),
    ),
    path = "/files",
    request_body(content_type = "multipart/form-data", content = inline(UploadFile), description = "File to upload"),
    responses(
        (status = 200, description = "OK")
    )
)]
pub async fn upload_file(
  State(state): State<Arc<AppState>>,
  UserToken(token): UserToken,
  mut multipart: Multipart,
) -> Result<Json<FileResponse>, ApiError> {
  let conn = &mut state
    .db_pool
    .get()
    .map_err(|err| ApiError::DatabaseError(DBError::ConnectionError(err)))?;
  super::common::check_user_exists(conn, token).await?;
  let mut file = None;
  loop {
    let next_field = multipart.next_field().await;
    if let Err(ref err) = next_field {
      tracing::debug!("No more next multipart field : {}", err.to_string());
      break;
    }
    if let Some(field) = next_field.unwrap() {
      let name = field.name().unwrap().to_string();
      if field.content_type().is_none() {
        return Err(ApiError::MissingField("Content-type header".to_owned()));
      }
      let content_type = field.content_type().unwrap();
      tracing::debug!("File received with content type: {content_type}");

      if name == "file" {
        let file_name = field.file_name().unwrap().to_owned();
        file = Some((file_name, content_type.to_owned(), field))
      }
    } else {
      break;
    }
  }

  if file.is_none() {
    return Err(ApiError::MissingField("file".to_owned()));
  }

  let file = file.unwrap();
  stream_to_file(&file.0, &file.1, file.2).await
}

async fn stream_to_file<S, E>(
  file_name: &str,
  content_type: &str,
  stream: S,
) -> Result<Json<FileResponse>, ApiError>
where
  S: Stream<Item = Result<Bytes, E>>,
  E: Into<BoxError>,
{
  async {
    // Convert the stream into an `AsyncRead`.
    let body_with_io_error = stream.map_err(|err| io::Error::new(io::ErrorKind::Other, err));
    let body_reader = StreamReader::new(body_with_io_error);
    futures::pin_mut!(body_reader);

    // Create the file. `File` implements `AsyncWrite`.
    let new_file_name = generate_file_name_with_timestamp(file_name);
    let path = std::path::Path::new(UPLOADS_DIRECTORY).join(&new_file_name);
    let mut file = BufWriter::new(File::create(&path).await?);

    // Copy the body into the file.
    tokio::io::copy(&mut body_reader, &mut file).await?;
    let file_url = format!(
      "{server_url}/files/{file_path}",
      server_url = get_server_url(),
      file_path = new_file_name
    );
    let file_response = FileResponse {
      name: new_file_name,
      content_type: content_type.into(),
      file_path: file_url,
    };
    Ok(Json(file_response))
  }
  .map_err(|err: io::Error| {
    tracing::error!(
      "An error occur when transmute stream to file: {}",
      err.to_string()
    );
    ApiError::Unknown
  })
  .await
}
