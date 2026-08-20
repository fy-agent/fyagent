import anthropicIconUrl from "./anthropic.svg";
import bytedanceIconUrl from "./bytedance.svg";
import chatglmIconUrl from "./chatglm.svg";
import claudeIconUrl from "./claude.svg";
import cohereIconUrl from "./cohere.svg";
import deepseekIconUrl from "./deepseek.svg";
import doubaoIconUrl from "./doubao.svg";
import geminiIconUrl from "./gemini.svg";
import googleIconUrl from "./google.svg";
import grokIconUrl from "./grok.svg";
import huggingfaceIconUrl from "./huggingface.svg";
import kimiIconUrl from "./kimi.svg";
import metaIconUrl from "./meta.svg";
import minimaxIconUrl from "./minimax.svg";
import mistralIconUrl from "./mistral.svg";
import nvidiaIconUrl from "./nvidia.svg";
import ollamaIconUrl from "./ollama.svg";
import openaiIconUrl from "./openai.svg";
import openrouterIconUrl from "./openrouter.svg";
import perplexityIconUrl from "./perplexity.svg";
import qwenIconUrl from "./qwen.svg";
import unknownIconUrl from "./unknown.svg";
import xaiIconUrl from "./xai.svg";

const OWNER_ALIASES: ReadonlyArray<{
  pattern: RegExp;
  url: string;
}> = [
  { pattern: /openai/i, url: openaiIconUrl },
  { pattern: /anthropic/i, url: anthropicIconUrl },
  { pattern: /claude/i, url: claudeIconUrl },
  { pattern: /deepseek/i, url: deepseekIconUrl },
  { pattern: /(qwen|alibaba|dashscope|tongyi)/i, url: qwenIconUrl },
  { pattern: /gemini/i, url: geminiIconUrl },
  { pattern: /google/i, url: googleIconUrl },
  { pattern: /grok/i, url: grokIconUrl },
  { pattern: /x[-.]?ai/i, url: xaiIconUrl },
  { pattern: /(kimi|moonshot)/i, url: kimiIconUrl },
  { pattern: /minimax/i, url: minimaxIconUrl },
  { pattern: /mistral/i, url: mistralIconUrl },
  { pattern: /(meta|llama)/i, url: metaIconUrl },
  { pattern: /doubao/i, url: doubaoIconUrl },
  { pattern: /(bytedance|volcengine|byteplus)/i, url: bytedanceIconUrl },
  { pattern: /(chatglm|zhipu|\bglm\b)/i, url: chatglmIconUrl },
  { pattern: /ollama/i, url: ollamaIconUrl },
  { pattern: /openrouter/i, url: openrouterIconUrl },
  { pattern: /(hugging\s*face|^hf$)/i, url: huggingfaceIconUrl },
  { pattern: /perplexity/i, url: perplexityIconUrl },
  { pattern: /cohere/i, url: cohereIconUrl },
  { pattern: /nvidia/i, url: nvidiaIconUrl },
];

const ID_PREFIXES: ReadonlyArray<{
  pattern: RegExp;
  url: string;
}> = [
  {
    pattern: /^(gpt|chatgpt|o[1-9]|davinci|text-embedding)/i,
    url: openaiIconUrl,
  },
  { pattern: /^claude/i, url: claudeIconUrl },
  { pattern: /^anthropic/i, url: anthropicIconUrl },
  { pattern: /^deepseek/i, url: deepseekIconUrl },
  { pattern: /^(qwen|qwq|qvq)/i, url: qwenIconUrl },
  { pattern: /^(gemini|gemma)/i, url: geminiIconUrl },
  { pattern: /^grok/i, url: grokIconUrl },
  { pattern: /^(kimi|moonshot)/i, url: kimiIconUrl },
  { pattern: /^minimax/i, url: minimaxIconUrl },
  { pattern: /^(mistral|mixtral|codestral|pixtral)/i, url: mistralIconUrl },
  { pattern: /^llama/i, url: metaIconUrl },
  { pattern: /^(glm|chatglm)/i, url: chatglmIconUrl },
  { pattern: /^(doubao|seed-|ep-)/i, url: doubaoIconUrl },
  { pattern: /^ollama/i, url: ollamaIconUrl },
  { pattern: /^openrouter/i, url: openrouterIconUrl },
  { pattern: /^(sonar|perplexity)/i, url: perplexityIconUrl },
  { pattern: /^(command|cohere)/i, url: cohereIconUrl },
  { pattern: /^(nvidia|nemotron)/i, url: nvidiaIconUrl },
];

function modelLeaf(modelId: string): string {
  const separator = modelId.lastIndexOf("/");
  return separator === -1 ? modelId : modelId.slice(separator + 1);
}

function isRemoteUrl(value: string): boolean {
  return /^https?:/i.test(value);
}

export function resolveModelVendorIcon(
  modelId: string,
  ownedBy?: string | null,
): string {
  const owned = ownedBy?.trim();
  if (owned) {
    for (const { pattern, url } of OWNER_ALIASES) {
      if (pattern.test(owned) && !isRemoteUrl(url)) return url;
    }
  }

  const trimmed = modelId.trim();
  const leaf = modelLeaf(trimmed);
  for (const { pattern, url } of ID_PREFIXES) {
    if ((pattern.test(leaf) || pattern.test(trimmed)) && !isRemoteUrl(url)) {
      return url;
    }
  }

  return unknownIconUrl;
}

export const unknownModelVendorIconUrl = unknownIconUrl;
