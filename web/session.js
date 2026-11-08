// Whether writing is unlocked: the server decides, and after a restart of modbusio it is not.

import { api } from "./api.js";
import { attempt, state } from "./store.js";

export async function refreshSession() {
  const session = await attempt(api.session);
  if (session) state.unlocked = session.unlocked;
}
