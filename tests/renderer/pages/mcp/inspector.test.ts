import { describe, expect, it } from "vitest";
import {
  buildMcpInspectorConfig,
  MCP_INSPECTOR_INITIALIZE_COMMAND,
  MCP_INSPECTOR_TOOLS_COMMAND,
} from "@/pages/mcp/inspector";

describe("local Inspector configuration", () => {
  it("exports only stdio launch fields and redacts credentials and preserves literal arguments", () => {
    const spec = {
      command: "node",
      args: ["server.js", "$(literal)"],
      env: { API_KEY: "fixture-secret" },
      cwd: "C:\\MCP",
      extra: "unused",
    };
    expect(JSON.parse(buildMcpInspectorConfig(spec))).toEqual({
      mcpServers: {
        "fyagent-diagnostic": {
          type: "stdio",
          command: "node",
          args: spec.args,
          env: { API_KEY: "••••••" },
          cwd: spec.cwd,
          connectionTimeout: 15000,
          requestTimeout: 15000,
        },
      },
    });
    expect(spec.extra).toBe("unused");
  });
  it.each(["http", "sse"] as const)(
    "exports %s endpoints and authorization headers without stdio fields",
    (type) => {
      expect(
        JSON.parse(
          buildMcpInspectorConfig({
            type,
            url: "http://localhost:3001/mcp",
            headers: { Authorization: "Bearer fixture-secret" },
            command: "unused",
          }),
        ),
      ).toEqual({
        mcpServers: {
          "fyagent-diagnostic": {
            type,
            url: "http://localhost:3001/mcp",
            headers: { Authorization: "••••••" },
            connectionTimeout: 15000,
            requestTimeout: 15000,
          },
        },
      });
    },
  );
  it("never exports plaintext secrets from env, headers, URLs or arguments and leaves the source intact", () => {
    const stdio = {
      command: "node",
      args: [
        "server.js",
        "--authToken=arg-canary",
        "--private-key",
        "private-canary",
        "sk-standalone-canary",
        "API_KEY=assignment-canary",
        "$(literal)",
      ],
      env: { ORDINARY_NAME: "env-canary" },
    };
    const remote = {
      type: "http" as const,
      url: "https://user-canary:pass-canary@example.com/mcp?sessionToken=query-canary&key=key-canary&region=cn",
      headers: {
        Authorization: "header-canary",
        Cookie: "cookie-canary",
        "X-API-Key": "api-canary",
        Accept: "application/json",
      },
    };
    const original = structuredClone({ stdio, remote });
    const output =
      buildMcpInspectorConfig(stdio) + buildMcpInspectorConfig(remote);
    for (const secret of [
      "arg-canary",
      "private-canary",
      "sk-standalone-canary",
      "env-canary",
      "assignment-canary",
      "user-canary",
      "pass-canary",
      "query-canary",
      "key-canary",
      "header-canary",
      "cookie-canary",
      "api-canary",
    ]) {
      expect(output).not.toContain(secret);
    }
    expect(output).toContain("region=cn");
    expect(output).toContain("application/json");
    expect(output).toContain("$(literal)");
    expect({ stdio, remote }).toEqual(original);
  });
  it("redacts plural credentials, embedded Bearer headers and URL fragments without changing the source", () => {
    const stdio = {
      command: "node",
      args: [
        "server.js",
        "--credentials",
        "credential-canary",
        "--credentials=inline-credential-canary",
        "--header",
        "Authorization: Bearer header-arg-canary",
        "prefix Authorization: Bearer embedded-header-canary",
        "$(literal)",
      ],
    };
    const remote = {
      type: "http" as const,
      url: "https://example.com/mcp?credentials=query-credential-canary&region=cn#access_token=fragment-canary&section=tools",
      headers: { "X-Auth": "Bearer xauth-canary", Accept: "application/json" },
    };
    const original = structuredClone({ stdio, remote });
    const output =
      buildMcpInspectorConfig(stdio) + buildMcpInspectorConfig(remote);
    for (const secret of [
      "credential-canary",
      "inline-credential-canary",
      "header-arg-canary",
      "embedded-header-canary",
      "xauth-canary",
      "query-credential-canary",
      "fragment-canary",
    ]) {
      expect(output).not.toContain(secret);
    }
    expect(output).toContain("--credentials");
    expect(output).toContain("--header");
    expect(output).toContain("$(literal)");
    expect(output).toContain("region=cn");
    expect(output).toContain("section=tools");
    expect(output).toContain("application/json");
    expect({ stdio, remote }).toEqual(original);
  });
  it.each([
    "https://example.com/mcp#access_token=fragment-only-canary",
    "not-a-url?credentials=fallback-query-canary#access_token=fallback-fragment-canary",
  ])("redacts fragment credentials in %s", (url) => {
    const spec = { type: "sse" as const, url };
    const original = structuredClone(spec);
    const output = buildMcpInspectorConfig(spec);
    expect(output).not.toContain("canary");
    expect(output).toContain("access_token=••••••");
    expect(spec).toEqual(original);
  });
  it("redacts X-Auth header values in arguments", () => {
    const output = buildMcpInspectorConfig({
      command: "node",
      args: [
        "--header",
        "X-Auth: colon-xauth-canary",
        "X-Auth=equals-xauth-canary",
      ],
    });
    expect(output).not.toContain("colon-xauth-canary");
    expect(output).not.toContain("equals-xauth-canary");
    expect(output).toContain("--header");
  });
  it("redacts Cookie arguments and embedded request headers", () => {
    const output = buildMcpInspectorConfig({
      command: "node",
      args: [
        "--cookie",
        "session=cookie-flag-canary",
        "Cookie=inline-cookie-canary",
        "--header",
        "Cookie: session=cookie-header-canary",
      ],
    });
    for (const secret of [
      "cookie-flag-canary",
      "inline-cookie-canary",
      "cookie-header-canary",
    ]) {
      expect(output).not.toContain(secret);
    }
  });
  it.each([
    ["Authorization", "Basic cmV2aWV3LXVzZXI6cmV2aWV3LWJhc2ljLWNhbmFyeQ=="],
    ["X-Auth", "review-inline-xauth-canary"],
    ["Cookie", "session=review-inline-cookie-canary"],
  ])(
    "redacts inline --header=%s values without changing the source",
    (name, value) => {
      const spec = {
        command: "node",
        args: [
          "server.js",
          `--header=${name}: ${value}`,
          "--header=Accept: application/json",
        ],
      };
      const original = structuredClone(spec);
      const output = buildMcpInspectorConfig(spec);
      expect(output).not.toContain(value);
      expect(JSON.parse(output).mcpServers["fyagent-diagnostic"].args).toEqual([
        "server.js",
        `--header=${name}:••••••`,
        "--header=Accept: application/json",
      ]);
      expect(spec).toEqual(original);
    },
  );
  it.each([
    [
      ["--passwd=passwd-equals-canary"],
      ["--passwd=••••••"],
      "passwd-equals-canary",
    ],
    [
      ["--passwd", "passwd-separate-canary"],
      ["--passwd", "••••••"],
      "passwd-separate-canary",
    ],
    [
      ["--passphrase=passphrase-equals-canary"],
      ["--passphrase=••••••"],
      "passphrase-equals-canary",
    ],
    [
      ["--passphrase", "passphrase-separate-canary"],
      ["--passphrase", "••••••"],
      "passphrase-separate-canary",
    ],
  ])(
    "redacts explicit password arguments %j without changing the source",
    (args, redacted, canary) => {
      const spec = {
        command: "node",
        args: ["server.js", ...args, "--header=Accept: application/json"],
      };
      const original = structuredClone(spec);
      const output = buildMcpInspectorConfig(spec);
      expect(output).not.toContain(canary);
      expect(JSON.parse(output).mcpServers["fyagent-diagnostic"].args).toEqual([
        "server.js",
        ...redacted,
        "--header=Accept: application/json",
      ]);
      expect(spec).toEqual(original);
    },
  );
  it.each([
    ["Authorization", "Basic cmV2aWV3LXVzZXI6cmV2aWV3LWJhc2ljLWNhbmFyeQ=="],
    ["X-Auth", "review-short-xauth-canary"],
    ["Cookie", "session=review-short-cookie-canary"],
  ])("redacts short -H%s values without changing the source", (name, value) => {
    const spec = {
      command: "node",
      args: [
        "server.js",
        `-H${name}: ${value}`,
        "-HAccept: application/json",
        "--header=Accept: application/json",
      ],
    };
    const original = structuredClone(spec);
    const output = buildMcpInspectorConfig(spec);
    expect(output).not.toContain(value);
    expect(JSON.parse(output).mcpServers["fyagent-diagnostic"].args).toEqual([
      "server.js",
      `-H${name}:••••••`,
      "-HAccept: application/json",
      "--header=Accept: application/json",
    ]);
    expect(spec).toEqual(original);
  });
  it.each(
    ["passwd", "passphrase"].flatMap((name) => [
      [
        `https://example.com/mcp?${name}=review-${name}-query-canary&region=cn`,
        `https://example.com/mcp?${name}=••••••&region=cn`,
        `review-${name}-query-canary`,
      ],
      [
        `https://example.com/mcp#${name}=review-${name}-fragment-canary&section=tools`,
        `https://example.com/mcp#${name}=••••••&section=tools`,
        `review-${name}-fragment-canary`,
      ],
      [
        `not-a-url?${name}=review-${name}-fallback-canary&region=cn`,
        `not-a-url?${name}=••••••&region=cn`,
        `review-${name}-fallback-canary`,
      ],
    ]),
  )(
    "redacts password aliases in %s and preserves ordinary fields and the source",
    (url, redacted, canary) => {
      const spec = {
        type: "http" as const,
        url,
        headers: { Accept: "application/json" },
      };
      const original = structuredClone(spec);
      const output = buildMcpInspectorConfig(spec);
      const server = JSON.parse(output).mcpServers["fyagent-diagnostic"];
      expect(output).not.toContain(canary);
      expect(server.url).toBe(redacted);
      expect(server.headers).toEqual({ Accept: "application/json" });
      expect(spec).toEqual(original);
    },
  );
  it.each(["passwd", "passphrase"])(
    "redacts %s assignments and preserves ordinary arguments and the source",
    (name) => {
      const canary = `review-${name}-assignment-canary`;
      const spec = {
        command: "node",
        args: [
          "server.js",
          `${name.toUpperCase()}=${canary}`,
          "--header=Accept: application/json",
          "-HAccept: application/json",
          "--header",
          "Accept: application/json",
        ],
      };
      const original = structuredClone(spec);
      const output = buildMcpInspectorConfig(spec);
      expect(output).not.toContain(canary);
      expect(JSON.parse(output).mcpServers["fyagent-diagnostic"].args).toEqual([
        "server.js",
        `${name.toUpperCase()}=••••••`,
        "--header=Accept: application/json",
        "-HAccept: application/json",
        "--header",
        "Accept: application/json",
      ]);
      expect(spec).toEqual(original);
    },
  );
  it.each(
    [
      "DB_PASS",
      "DB_PWD",
      "DB_CREDS",
      "db-pass",
      "db-pwd",
      "db-creds",
      "Cookie",
      "Proxy-Authorization",
    ].flatMap((name) =>
      [
        ["https://example.com/mcp?", "region=cn"],
        ["https://example.com/mcp#", "section=tools"],
        ["not-a-url?", "region=cn"],
      ].map(([prefix, ordinary]) => {
        const canary = `review-${name}-url-canary`;
        const tail = `&${ordinary}&PWD=work&OLDPWD=old&BYPASS=enabled&DBPASS=ordinary&DBPWD=ordinary&DBCREDS=ordinary`;
        return [
          `${prefix}${name}=${canary}${tail}`,
          `${prefix}${name}=••••••${tail}`,
          canary,
        ];
      }),
    ),
  )(
    "redacts credential aliases in %s and preserves ordinary fields and the source",
    (url, redacted, canary) => {
      const spec = {
        type: "http" as const,
        url,
        headers: { Accept: "application/json" },
      };
      const original = structuredClone(spec);
      const output = buildMcpInspectorConfig(spec);
      const server = JSON.parse(output).mcpServers["fyagent-diagnostic"];
      expect(output).not.toContain(canary);
      expect(server.url).toBe(redacted);
      expect(server.headers).toEqual({ Accept: "application/json" });
      expect(spec).toEqual(original);
    },
  );
  it.each(
    ["DB_PASS", "DB_PWD", "DB_CREDS", "db-pass", "db-pwd", "db-creds"].flatMap(
      (name) => {
        const canary = `review-${name}-argument-canary`;
        const flag = `--${name.replace(/_/g, "-")}`;
        return [
          [[`${name}=${canary}`], [`${name}=••••••`], canary],
          [[flag, canary], [flag, "••••••"], canary],
        ] as const;
      },
    ),
  )(
    "redacts credential arguments %j and preserves ordinary arguments and the source",
    (args, redacted, canary) => {
      const ordinary = [
        "--header=Accept: application/json",
        "-HAccept: application/json",
        "--header",
        "Accept: application/json",
        "PWD=/work",
        "OLDPWD=/old",
        "BYPASS=enabled",
        "DBPASS=ordinary",
        "DBPWD=ordinary",
        "DBCREDS=ordinary",
        "--pwd",
        "/work",
        "--oldpwd",
        "/old",
        "--bypass",
        "enabled",
        "--dbpass",
        "ordinary",
        "--dbpwd",
        "ordinary",
        "--dbcreds",
        "ordinary",
      ];
      const spec = {
        command: "node",
        args: ["server.js", ...args, ...ordinary],
      };
      const original = structuredClone(spec);
      const output = buildMcpInspectorConfig(spec);
      expect(output).not.toContain(canary);
      expect(JSON.parse(output).mcpServers["fyagent-diagnostic"].args).toEqual([
        "server.js",
        ...redacted,
        ...ordinary,
      ]);
      expect(spec).toEqual(original);
    },
  );
  it.each(
    [
      "SERVICE_PAT",
      "service-pat",
      "Set-Cookie",
      "X-Auth",
      "%53ERVICE%5FPAT",
      "Set%2DCookie",
      "X%2DAuth",
    ].flatMap((name) =>
      [
        ["https://example.com/mcp?", "region=cn"],
        ["https://example.com/mcp#", "section=tools"],
        ["not-a-url?", "region=cn"],
      ].map(([prefix, ordinary]) => {
        const canary = `review-${name}-url-canary`;
        const tail = `&${ordinary}&PWD=work&OLDPWD=old&SERVICEPAT=ordinary&servicepat=ordinary&SPAT=ordinary`;
        const key =
          prefix.endsWith("?") && prefix.startsWith("https:")
            ? encodeURIComponent(decodeURIComponent(name))
            : name;
        return [
          `${prefix}${name}=${canary}${tail}`,
          `${prefix}${key}=••••••${tail}`,
          canary,
        ];
      }),
    ),
  )(
    "redacts PAT and sensitive header parameters in %s without changing ordinary fields or the source",
    (url, redacted, canary) => {
      const spec = {
        type: "http" as const,
        url,
        headers: { Accept: "application/json" },
      };
      const original = structuredClone(spec);
      const output = buildMcpInspectorConfig(spec);
      const server = JSON.parse(output).mcpServers["fyagent-diagnostic"];
      expect(output).not.toContain(canary);
      expect(server.url).toBe(redacted);
      expect(server.headers).toEqual({ Accept: "application/json" });
      expect(spec).toEqual(original);
    },
  );
  it.each(
    ["SERVICE_PAT", "service-pat"].flatMap((name) => {
      const canary = `review-${name}-argument-canary`;
      const flag = `--${name.replace(/_/g, "-")}`;
      return [
        [[`${name}=${canary}`], [`${name}=••••••`], canary],
        [[flag, canary], [flag, "••••••"], canary],
      ] as const;
    }),
  )(
    "redacts PAT arguments %j without changing ordinary arguments or the source",
    (args, redacted, canary) => {
      const ordinary = [
        "--header=Accept: application/json",
        "-HAccept: application/json",
        "--header",
        "Accept: application/json",
        "PWD=/work",
        "OLDPWD=/old",
        "SERVICEPAT=ordinary",
        "servicepat=ordinary",
        "SPAT=ordinary",
        "--pwd",
        "/work",
        "--oldpwd",
        "/old",
        "--servicepat",
        "ordinary",
        "--spat",
        "ordinary",
      ];
      const spec = {
        command: "node",
        args: ["server.js", ...args, ...ordinary],
      };
      const original = structuredClone(spec);
      const output = buildMcpInspectorConfig(spec);
      expect(output).not.toContain(canary);
      expect(JSON.parse(output).mcpServers["fyagent-diagnostic"].args).toEqual([
        "server.js",
        ...redacted,
        ...ordinary,
      ]);
      expect(spec).toEqual(original);
    },
  );
  it.each(["SERVICE_PAT", "service-pat"])(
    "redacts %s object headers without changing ordinary headers or the source",
    (name) => {
      const canary = `review-${name}-header-canary`;
      const ordinary = {
        Accept: "application/json",
        PWD: "/work",
        OLDPWD: "/old",
        SERVICEPAT: "ordinary",
        SPAT: "ordinary",
      };
      const spec = {
        type: "http" as const,
        url: "https://example.com/mcp",
        headers: { [name]: canary, ...ordinary },
      };
      const original = structuredClone(spec);
      const output = buildMcpInspectorConfig(spec);
      const server = JSON.parse(output).mcpServers["fyagent-diagnostic"];
      expect(output).not.toContain(canary);
      expect(server.url).toBe(spec.url);
      expect(server.headers).toEqual({ [name]: "••••••", ...ordinary });
      expect(spec).toEqual(original);
    },
  );
  describe("credential names with descriptive suffixes", () => {
    const names = [
      "clientSecretValue",
      "PASSWORD_VALUE",
      "PRIVATE_KEY_PEM",
      "PASSWD_VALUE",
      "credentialFile",
      "bearerTokenValue",
      "sessionToken",
      "service-key",
    ];
    const ordinary = {
      PWD: "work",
      OLDPWD: "old",
      SERVICEPAT: "ordinary",
      SPAT: "ordinary",
      maxTokens: "4096",
      tokenizer: "ordinary",
      keyboardLayout: "us",
      baseUrl: "ordinary",
      model: "ordinary",
      BYPASS: "enabled",
      DBPASS: "ordinary",
      DBPWD: "ordinary",
      DBCREDS: "ordinary",
    };
    it.each(
      names.flatMap((name) =>
        [
          ["https://example.com/mcp?", "region=cn"],
          ["https://example.com/mcp#", "section=tools"],
          ["not-a-url?", "region=cn"],
        ].map(([prefix, field]) => [name, prefix, field]),
      ),
    )(
      "redacts %s in %s and preserves ordinary fields and the source",
      (name, prefix, field) => {
        const canary = `review-${name}-url-canary`;
        const tail = Object.entries(ordinary)
          .map(([key, value]) => `&${key}=${value}`)
          .join("");
        const spec = {
          type: "http" as const,
          url: `${prefix}${name}=${canary}&${field}${tail}`,
          headers: { Accept: "application/json" },
        };
        const original = structuredClone(spec);
        const output = buildMcpInspectorConfig(spec);
        const server = JSON.parse(output).mcpServers["fyagent-diagnostic"];
        expect(output).not.toContain(canary);
        expect(server.url).toBe(`${prefix}${name}=••••••&${field}${tail}`);
        expect(server.headers).toEqual({ Accept: "application/json" });
        expect(spec).toEqual(original);
      },
    );
    it.each(names)(
      "redacts %s object headers and preserves ordinary headers and the source",
      (name) => {
        const canary = `review-${name}-header-canary`;
        const headers = {
          [name]: canary,
          Accept: "application/json",
          ...ordinary,
        };
        const spec = {
          type: "http" as const,
          url: "https://example.com/mcp",
          headers,
        };
        const original = structuredClone(spec);
        const output = buildMcpInspectorConfig(spec);
        const server = JSON.parse(output).mcpServers["fyagent-diagnostic"];
        expect(output).not.toContain(canary);
        expect(server.url).toBe(spec.url);
        expect(server.headers).toEqual({
          [name]: "••••••",
          Accept: "application/json",
          ...ordinary,
        });
        expect(spec).toEqual(original);
      },
    );
    it.each(
      names.flatMap((name) => [
        [name, "assignment"],
        [name, "CLI"],
      ]),
    )(
      "redacts %s %s arguments and preserves ordinary arguments and the source",
      (name, entry) => {
        const canary = `review-${name}-argument-canary`;
        const ordinaryArgs = Object.entries(ordinary).flatMap(
          ([key, value]) => [`${key}=${value}`, `--${key}`, value],
        );
        const args =
          entry === "assignment"
            ? [`${name}=${canary}`]
            : [`--${name}`, canary];
        const redacted =
          entry === "assignment" ? [`${name}=••••••`] : [`--${name}`, "••••••"];
        const spec = {
          command: "node",
          args: ["server.js", ...args, ...ordinaryArgs],
        };
        const original = structuredClone(spec);
        const output = buildMcpInspectorConfig(spec);
        expect(output).not.toContain(canary);
        expect(
          JSON.parse(output).mcpServers["fyagent-diagnostic"].args,
        ).toEqual(["server.js", ...redacted, ...ordinaryArgs]);
        expect(spec).toEqual(original);
      },
    );
  });
  it("normalizes authentication header names before redacting values", () => {
    const output = buildMcpInspectorConfig({
      type: "http",
      url: "https://example.com/mcp",
      headers: {
        " X_Auth_Custom ": "custom-auth-canary",
        AUTHORIZATION: "authorization-canary",
        "X-Session-Token": "token-header-canary",
        "X-Service-Key": "key-header-canary",
        " CoOkIe ": "cookie-record-canary",
        SET_COOKIE: "set-cookie-canary",
        Accept: "application/json",
      },
    });
    for (const secret of [
      "custom-auth-canary",
      "authorization-canary",
      "token-header-canary",
      "key-header-canary",
      "cookie-record-canary",
      "set-cookie-canary",
    ]) {
      expect(output).not.toContain(secret);
    }
    expect(output).toContain("application/json");
  });
  it("redacts percent-encoded query parameter names", () => {
    for (const key of [
      "%61ccess_token",
      "access%5Ftoken",
      "%41cCeSs%5ftOkEn",
      "access_%74oken",
      "%61%63%63%65%73%73%5F%74%6F%6B%65%6E",
    ]) {
      const output = buildMcpInspectorConfig({
        type: "http",
        url: `https://example.com/mcp?${key}=encoded-query-canary&region=cn`,
      });
      expect(output).not.toContain("encoded-query-canary");
      expect(output).toContain("region=cn");
    }
  });
  it("redacts percent-encoded fragment and fallback parameter names", () => {
    for (const key of [
      "%61ccess_token",
      "access%5Ftoken",
      "%41cCeSs%5ftOkEn",
      "access_%74oken",
      "%61%63%63%65%73%73%5F%74%6F%6B%65%6E",
    ]) {
      for (const url of [
        `https://example.com/mcp#${key}=encoded-fragment-canary&section=tools`,
        `not-a-url?${key}=encoded-fallback-query-canary#${key}=encoded-fragment-canary&section=tools`,
      ]) {
        const output = buildMcpInspectorConfig({ type: "sse", url });
        expect(output).not.toContain("encoded-fragment-canary");
        expect(output).not.toContain("encoded-fallback-query-canary");
        expect(output).toContain("section=tools");
      }
    }
  });
  it("rejects missing launch information and exposes only initialize and tools/list commands", () => {
    expect(() => buildMcpInspectorConfig({})).toThrow(
      "请先补全 MCP 命令或地址",
    );
    expect(() => buildMcpInspectorConfig({ type: "http" })).toThrow(
      "请先补全 MCP 命令或地址",
    );
    expect(MCP_INSPECTOR_INITIALIZE_COMMAND).toContain("--method initialize");
    expect(MCP_INSPECTOR_TOOLS_COMMAND).toContain("--method tools/list");
    expect(MCP_INSPECTOR_TOOLS_COMMAND).not.toContain("tools/call");
  });
});
