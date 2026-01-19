import {
  Accessor,
  Component, 
  JSX,   
  createContext,
  createEffect,
  createSignal,
  Show,
} from 'solid-js';

export interface UserStoreContextProps {
  children: JSX.Element;
}

export interface UserStore {
  //user: Accessor<UserDTO | null> | null;
  //setUser: (user: UserDTO | null) => void;
  login: () => void;
  logout: () => void;
}

export const UserContext = createContext<UserStore>({
  login: () => {},
  logout: () => {},
});

export const UserContextComp = (props: any) => {

  const [user, setUser] = createSignal<string | null>(null);

  const login = () => {
    const apiHost: string = "http://localhost:8080/api";
    fetch(`${apiHost}/auth/me`, {
      credentials: "include",
    })
    .then((res) => {
      if (res.status === 401) {
        window.location.href = `${apiHost}/auth?redirect_uri=${window.origin}`;
        setUser(null);
        return null;
      }
      return res.json();
    })
    .then((data) => {
      if (isUserDTO(data)) {
        console.log("Setting user:", JSON.stringify(data));
        setUser(data?.email);
      }
    })
    .catch((err) => {
      console.error(err);
    });
  };

  const isUserDTO = (data: any) => {
    return true;
  };

  const userStore: UserStore = {
    login: login,
    logout: () => {},
  };
    
  // This is meant to call login when the dependency signal user() changes. 
  // But I cannot see where it can be changed outside the login itself (i.e. SetUser called).
  // (See the SetUser and invalidate functions in the original code.)
  // There is also logout() which clears 'user'. 
  // W/o that above, this effect I think does not help much: 
  // in fact it is only called first when the signal is created (user == null) 
  // and second time immediately called after login() calls SetUser() (so login() is called twice)
  createEffect(() => {
    console.log("starting login", user());
    void login();
  });

  return (
    <div>
      <label style="margin-right: 5px">User:</label>
      <Show when={user()} fallback={<>Not Logged</>}>{(user) => <>{user()}</>}</Show>
      <UserContext.Provider value={userStore}>
        {props.children}
      </UserContext.Provider>
    </div>
  );

};