import Genius, {
  InvalidGeniusKeyError,
  NoResultError,
} from "genius-lyrics";
import { Application, Context, Router, Status } from "@oak/oak";

const router = new Router();

function notFound(context: Context) {
  context.response.status = Status.NotFound;
  context.response.body = { error: "NotFound" };
}

function unauthorized(context: Context) {
  context.response.status = Status.Unauthorized;
  context.response.body = { error: "Unauthorized" };
}

router.get("/:id/lyrics", async (ctx) => {
  try {
    const auth = ctx.request.headers.get("Authorization") as string;
    if (!auth) {
      return unauthorized(ctx);
    }
    const genius = new Genius.Client(auth);

    const song = await genius.songs.get(parseInt(ctx.params.id));
    ctx.response.body = { lyrics: await song.lyrics() };
    ctx.response.type = "json";
  } catch (err) {
    if (err instanceof NoResultError) {
      notFound(ctx);
    } else if (err instanceof InvalidGeniusKeyError) {
      unauthorized(ctx);
    } else {
      console.log(err);
      throw err;
    }
  }
});

router.get("/search", async (ctx) => {
  try {
    const auth = ctx.request.headers.get("Authorization") as string;
    if (!auth) {
      return unauthorized(ctx);
    }

    const genius = new Genius.Client(auth);

    const query = ctx.request.url.searchParams.get("q");
    const songs = await genius.songs.search(query as string);

    ctx.response.body = JSON.stringify(songs, (_key, val) => {
      return val instanceof Genius.Client ? undefined : val;
    });

    ctx.response.type = "json";
  } catch (err) {
    if (err instanceof NoResultError) {
      notFound(ctx);
    } else if (err instanceof InvalidGeniusKeyError) {
      unauthorized(ctx);
    } else {
      console.log(err);
      throw err;
    }
  }
});

const app = new Application();
app.use(router.routes());
app.use(router.allowedMethods());

app.listen({ port: 8090 });
