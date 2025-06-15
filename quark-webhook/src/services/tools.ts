import axios from 'axios';

export const tools = axios.create({
  baseURL: process.env.API_TOOLS_URL,
  headers: {
    'Content-Type': 'application/json',
  },
});
