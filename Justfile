set dotenv-load := true
set positional-arguments

just := quote(just_executable())
this := just + " -f " + quote(justfile())

server := env_var_or_default('DEPLOY_DESTINATION', 'rustify')
path := env_var_or_default("DEPLOY_PATH", "/srv/rustify")

import-db:
  ssh {{ server }} "docker compose --project-directory /srv/rustify exec db pg_dump -U rustify --no-owner --clean --if-exists" | docker compose exec -T db psql -U postgres
  docker compose exec db psql -U postgres -c "update \"user\" set status='pending'; delete from spotify_auth"

github-workflow-deploy:
  gh workflow run deploy.yml --ref "$(git rev-parse --abbrev-ref HEAD)"

update-secret:
  cat _infra/ansible/inventory/main/group_vars/all.yml | base64 -w 0 | gh secret set ANSIBLE_GROUP_VARS_ALL

sea-orm-generate:
  sea-orm-cli generate entity -o src/entity.example --expanded-format

validate:
  cargo clippy
  cargo test
  ansible-lint .infra/ansible/playbook.yml

fix:
  cargo fmt
  cargo clippy --fix --allow-dirty --allow-staged
  ansible-lint --write .infra/ansible/playbook.yml

compose +args:
  ssh {{ server }} -- docker-compose -f "{{ path }}/docker-compose.yml" {{ args }}

logs:
  {{ this }} compose logs -f

ssh:
  ssh -t "{{ server }}" "cd {{ path }}; bash --login"

restart:
  ssh "{{ server }}" -- docker-compose --project-directory "{{ path }}" restart bot track_check queues

run-bot:
  proxychains4 -q cargo run "bot"

run-track-check:
  proxychains4 -q cargo run "track-check"

run-queues:
  proxychains4 -q cargo run "queues"

run-server:
  proxychains4 -q cargo run "server"

run:
  parallel --tagstring "[{}]" --line-buffer -j4 --halt now,fail=1 just ::: "run-bot" "run-track-check" "run-queues" "run-server"

watch:
  cargo watch -s 'just run'
