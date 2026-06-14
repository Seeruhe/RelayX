type CommandBlockProps = {
  title: string;
  command: string;
};

export function CommandBlock({ title, command }: CommandBlockProps) {
  return (
    <figure className="command-block">
      <figcaption>{title}</figcaption>
      <pre><code>{command.trim()}</code></pre>
    </figure>
  );
}
