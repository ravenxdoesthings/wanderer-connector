create_config:
	cp wanderer-conf.env.sample wanderer-conf.env

base_key:
	sed -i '' "s|SECRET_KEY_BASE=.*|SECRET_KEY_BASE=$(shell openssl rand -base64 48)|" wanderer-conf.env

cloak_key:
	sed -i '' "s|CLOAK_KEY=.*|CLOAK_KEY=$(shell openssl rand -base64 32)|" wanderer-conf.env