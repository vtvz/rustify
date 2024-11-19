import {
  search,
  // import the result types you want
  OrganicResult,
  DictionaryResult,
  // helpful to import ResultTypes to filter results
  ResultTypes
} from 'google-sr';
import { JSDOM } from "jsdom";
import Genius, {
  InvalidGeniusKeyError,
  NoResultError,
} from "genius-lyrics";
import { Application, Context, Router, Status } from "@oak/oak";
import axios from "axios";

const router = new Router();

function notFound(context: Context) {
  context.response.status = Status.NotFound;
  context.response.body = { error: "NotFound" };
}

function unauthorized(context: Context) {
  context.response.status = Status.Unauthorized;
  context.response.body = { error: "Unauthorized" };
}

router.get("/genius/:id/lyrics", async (ctx) => {
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

router.get("/genius/search", async (ctx) => {
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


router.get("/azlyrics/search", async (ctx) => {
  const query = ctx.request.url.searchParams.get("q");

  const queryResult = await search({
    query: `site:www.azlyrics.com ${query} lyrics`,
    // OrganicResult is the default, however it is recommended to ALWAYS specify the result type
    resultTypes: [OrganicResult],
    // to add additional configuration to the request, use the requestConfig option
    // which accepts a AxiosRequestConfig object
    // OPTIONAL
    requestConfig: {
      params: {
        // enable "safe mode"
        safe: 'active'
      },
    },
  });

  ctx.response.body = JSON.stringify(queryResult);

  ctx.response.type = "json";
});

router.get("/azlyrics/lyrics", async (ctx) => {
  try {
    const query = ctx.request.url.searchParams.get("url");

    let resp = await axios.get(query as string);

    const dom = new JSDOM(resp.data);
    const selected = dom.window.document.querySelector('br + br + div')
    if (!selected) {
      return notFound(ctx);
    }

    ctx.response.body = { lyrics: selected.textContent?.trim() };
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
