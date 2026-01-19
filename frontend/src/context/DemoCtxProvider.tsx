//import React from "react";
import { MyContext } from "./DemoCtxCreate";

export function Provider(props: { children?: any; value: string }) {
  return (
    <MyContext.Provider value={props.value}>
      {props.children}
    </MyContext.Provider>
  );
}
