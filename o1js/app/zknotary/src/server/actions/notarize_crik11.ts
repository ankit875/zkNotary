"use server";

// Crik11 notarization

import { RootSchemaValuesType } from "@/lib/proof_types";

export type ServiceNames = "github" | "etherscan" | "crik11";

export type NotaryCrik11scanArgs = {
  match_id: string;
};

//const NOTARY_SERVER_HOST = process.env.NOTARY_PROVER_HOST!;
const NOTARY_SERVER_HOST = "127.0.0.1";
// const NOTARY_SERVER_HOST = "54.83.188.45";
const NOTARY_SERVER_PORT = 8080;

export async function notarize_crik11(args: NotaryCrik11scanArgs) {
  let { match_id } = args;

  let url = `http://${NOTARY_SERVER_HOST}:${NOTARY_SERVER_PORT}/notarize_crik11?match_id=${match_id}`;
    console.log("url", url);
  let response = await fetch(url);

  let jsonData = (await response.json()) as RootSchemaValuesType;

  console.log("jsonData", jsonData);

  return {
    data: jsonData,
  };
}
