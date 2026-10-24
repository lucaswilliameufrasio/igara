import { Plugin } from "@opencode/plugin"

export default Plugin.define({
  id: "igara.build-notify",
  async setup(ctx) {
    await ctx.tool.hook("execute.before", (event) => {
      if (event.tool !== "igara_remote_build") {
        return
      }

      const input = event.input as { host?: unknown }
      const host = typeof input.host === "string" ? input.host : "(unspecified)"
      console.info("[igara] remote build requested", { host })
    })
  },
})
