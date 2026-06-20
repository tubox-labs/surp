import { createOpenAICompatible } from "@ai-sdk/openai-compatible";

export function createAiGatewayProvider(apiKey: string) {
  const baseURL = process.env.AI_GATEWAY_URL || "https://generativelanguage.googleapis.com/v1beta/openai";
  return createOpenAICompatible({
    name: "ai-gateway",
    baseURL,
    headers: {
      Authorization: `Bearer ${apiKey}`,
    },
  });
}
