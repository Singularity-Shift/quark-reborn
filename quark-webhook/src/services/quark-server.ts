import axios from 'axios';

export const quarkServer = axios.create({
  baseURL: process.env.QUARK_SERVER_URL,
  headers: {
    'Content-Type': 'application/json',
  },
});
