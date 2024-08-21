"use client";

import React, { useCallback } from "react";

import { notarize_etherscan } from "@/server/actions/notarize_etherscan";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";

import { NOTARY_PUB_KEY } from "@/lib/constants";

import {
  Form,
  FormItem,
  FormField,
  FormLabel,
  FormControl,
  FormMessage,
  FormDescription,
} from "@/components/ui/form";

import { Button } from "@/components/ui/button";

import { z } from "zod";
import { toast } from "sonner";
import { Input } from "../ui/input";
import { useExamplesStore } from "@/stores/examples-store";
import { notarize_crik11 } from "@/server/actions/notarize_crik11";

const crik11FormSchema = z.object({
    match_id: z.string(),
});

export type Crik11FormSchema = z.infer<typeof crik11FormSchema>;

export default function Crik11Form() {
  const {
    setVerifiedData,
    setProofData: setNotorizedData,
    isFetching,
    setFetching,
  } = useExamplesStore((state) => state);

  const form = useForm<z.infer<typeof crik11FormSchema>>({
    resolver: zodResolver(crik11FormSchema),
    defaultValues: {
      match_id: "",
    },
  });

  const onSubmit = useCallback(
    async (values: z.infer<typeof crik11FormSchema>) => {
        console.log("values", values);
    //   const verify = (await import("zknotary-verifier")).verify;
      setFetching(true);

      toast.promise(() => notarize_crik11(values), {
        loading:
          "Notarizing Crik11 data. Please stay on this page, it can take up to a minute.",
        success: ({ data }) => {
          setFetching(false);
          setNotorizedData(data, "crik11");

          let json_data = JSON.stringify(data);

        //   let verifiedData = verify(json_data, NOTARY_PUB_KEY);

        //   console.log("verifiedData", verifiedData);

        //   setVerifiedData(verifiedData, "crik11");

          return "Data Notarized Successfully! Check formatted and Raw Data Tabs for more info.";
        },
        error: (error) => {
          setFetching(false);
          return "Error: " + error.message;
        },
      });
    },
    []
  );

  return (
    <Form {...form}>
      <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-5">
        <FormField
          control={form.control}
          name="match_id"
          render={({ field }) => (
            <FormItem>
              <FormLabel>Match Id</FormLabel>
              <FormControl>
                <Input placeholder="" {...field} />
              </FormControl>
              <FormDescription>
                Enter the match id of the Crik11 match you want to retrieve the data for.
              </FormDescription>
              <FormMessage />
            </FormItem>
          )}
        />
        <Button disabled={isFetching} type="submit">
          Notarize
        </Button>
      </form>
    </Form>
  );
}
