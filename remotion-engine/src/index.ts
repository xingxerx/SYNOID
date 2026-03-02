import { registerRoot } from "remotion";
import { MyComposition } from "./Composition";

// A basic composition registration for Remotion
registerRoot(() => {
  return (
    <>
      <MyComposition />
    </>
  );
});
