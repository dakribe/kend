import app/web
import gleam/http.{Get}
import gleam/string_tree
import wisp.{type Request, type Response}

pub fn handle_request(req: Request) -> Response {
  use req <- web.middleware(req)

  case wisp.path_segments(req) {
    [] -> hello_world(req)
    _ -> wisp.not_found()
  }
}

fn hello_world(req: Request) -> Response {
  use <- wisp.require_method(req, Get)
  let body = string_tree.from_string("{\"message\": \"Hello world\"}")
  wisp.ok() |> wisp.json_body(body)
}
