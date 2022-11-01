// Copyright Metatype under the Elastic License 2.0.

import { Runtime } from "./Runtime.ts";
import { ComputeStage } from "../engine.ts";
import { TypeNode } from "../type_node.ts";
import Chance from "chance";
// import { ensure } from "../utils.ts";
import { Resolver, RuntimeInitParams } from "../types.ts";

export class RandomRuntime extends Runtime {
  seed: number | null;
  chance: typeof Chance;

  constructor(seed: number | null) {
    super();
    this.seed = seed;
    if (this.seed == null) {
      this.chance = new Chance();
    } else {
      this.chance = new Chance(seed);
    }
  }

  static async init(params: RuntimeInitParams): Promise<Runtime> {
    const { seed } = params.args as {
      seed: number | null;
    };
    return await new RandomRuntime(seed);
  }

  async deinit(): Promise<void> {}

  materialize(
    stage: ComputeStage,
    waitlist: ComputeStage[],
    _verbose: boolean,
  ): ComputeStage[] {
    const stagesMat: ComputeStage[] = [];

    const sameRuntime = Runtime.collectRelativeStages(stage, waitlist);

    stagesMat.push(
      new ComputeStage({
        ...stage.props,
        resolver: this.execute(stage.props.outType),
        batcher: (x: any) => x,
      }),
    );

    stagesMat.push(...sameRuntime.map((stage) =>
      new ComputeStage({
        ...stage.props,
        dependencies: [...stage.props.dependencies, stagesMat[0].id()],
        resolver: this.execute(stage.props.outType),
      })
    ));

    return stagesMat;
  }

  execute(typ: TypeNode): Resolver {
    return () => {
      // if (Object.prototype.hasOwnProperty.call(typ.data, "random")) {
      //   const entries = Object.entries(
      //     typ.data.random as Record<string, unknown>,
      //   );
      //   ensure(
      //     entries.length === 1,
      //     `invalid random generation data ${typ.data.random}`,
      //   );
      //   const [[fn, arg]] = entries;
      //   return this.chance[fn](arg);
      // }

      switch (typ.type) {
        // case "struct":
        //   return {};
        // case "list":
        //   return [];
        // case "integer":
        //   return this.chance.integer();
        // case "unsigned_integer": {
        //   let n = this.chance.integer();
        //   while (n < 0) {
        //     n = this.chance.integer();
        //   }
        //   return n;
        // }
        // case "uuid":
        //   return this.chance.guid();
        // case "string":
        //   return this.chance.string();
        // case "email":
        //   return this.chance.email();
        // case "char":
        //   return this.chance.character();
        // case "boolean":
        //   return this.chance.bool();
        default:
          throw new Error(`type not supported "${typ.type}"`);
      }
    };
  }
}
