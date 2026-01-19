import type { Component, JSX } from 'solid-js';
import Comp from './Comp';

import { useContext } from "solid-js";
import { MyContext } from "./context/DemoCtxCreate";
import { UserContextComp, UserStoreContextProps } from "./context/UserContext";

const Provider = (props: UserStoreContextProps) => {
  const v1 = "new value1";

  return (
    <MyContext.Provider value={v1}>
      {props.children}
    </MyContext.Provider>
  );
};

const Child = () => {
  const value = useContext(MyContext);

  return (
    <>
      <span style="margin-right: 5px">My Value:</span>
      <span>{value}</span>
    </>
  );
};

const App: Component = () => {
  return (
    <>
      <h1>Demo:</h1>
      <Comp />
      <Provider>
        <Child />
      </Provider>
      <UserContextComp />
    </>
  );
};

export default App;
