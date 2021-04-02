import Head from "next/head";
import styles from "../styles/Home.module.css";

export default function Home() {
  return (
    <div className={styles.container}>
      <Head>
        <title>Create Next App</title>
        <link rel="icon" href="/favicon.ico" />
      </Head>

      <main className={styles.main}>
        <img src="icons/logo.svg" />

        <p className={styles.description}>Nothing Graphics Engine</p>

        <div className={styles.grid}>
          <a href="/app" className={styles.card}>
            <h3>Quick Start &rarr;</h3>
            <p>Open new empty editor</p>
          </a>

          <a href="https://nextjs.org/learn" className={styles.card}>
            <h3>Demos &rarr;</h3>
            <p>See what you can build with Nothing engine</p>
          </a>

          <a
            href="https://github.com/vercel/next.js/tree/master/examples"
            className={styles.card}
          >
            <h3>Docs &rarr;</h3>
            <p>View API Documentations</p>
          </a>

          <a
            href="https://vercel.com/new?utm_source=create-next-app&utm_medium=default-template&utm_campaign=create-next-app"
            className={styles.card}
          >
            <h3>Bridged &rarr;</h3>
            <p>
              Visit Bridged - A Nothing based Collaborative Realtime Graphics
              editor
            </p>
          </a>
        </div>
      </main>
    </div>
  );
}
