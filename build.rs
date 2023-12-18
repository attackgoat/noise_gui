use {
    anyhow::Context,
    lazy_static::lazy_static,
    rand::{seq::SliceRandom, thread_rng},
    std::{
        any::type_name, env::var, error::Error, fmt::Display, fs::File, io::Write, path::PathBuf,
    },
};

// We want a shuffled array of all u8's but we don't want to generate them at runtime.
// This data is used to load image chunks in a somewhat better looking order than by scan row.
// Also we want to use a number of these arrays so repeated/frequent changes don't appear similar.

const COUNT: usize = 32;

lazy_static! {
    static ref OUT_DIR: PathBuf = PathBuf::from(var("OUT_DIR").unwrap());
}

fn main() -> anyhow::Result<()> {
    let path = OUT_DIR.join("rand.rs");
    let mut file = File::create(&path).with_context(|| format!("Creating {}", path.display()))?;

    for idx in 0..COUNT {
        let arr = shuffled_array::<u8, 256>()?;

        write_slice(&mut file, &arr, format!("SHUFFLED_U8{idx}")).context("Writing data")?;
    }

    writeln!(&mut file, "static SHUFFLED_U8: &[&[u8]] = &[")?;

    for idx in 0..COUNT {
        writeln!(&mut file, "    &SHUFFLED_U8{idx},")?;
    }

    writeln!(&mut file, "];")?;

    writeln!(
        &mut file,
        "pub fn shuffled_u8(seed: usize) -> &'static [u8] {{"
    )?;
    writeln!(&mut file, "    SHUFFLED_U8[seed % {COUNT}]")?;
    writeln!(&mut file, "}}")?;

    Ok(())
}

fn shuffled_array<T, const N: usize>() -> anyhow::Result<[T; N]>
where
    T: Copy + Default + TryFrom<usize>,
    <T as TryFrom<usize>>::Error: Error + Send + Sync + 'static,
{
    let mut res = [Default::default(); N];

    for idx in 0..res.len() {
        res[idx] = idx
            .try_into()
            .with_context(|| format!("Converting {idx} to {}", type_name::<T>()))?;
    }

    res.shuffle(&mut thread_rng());

    Ok(res)
}

fn write_slice<T>(writer: &mut impl Write, slice: &[T], name: impl Display) -> anyhow::Result<()>
where
    T: Display,
{
    write!(
        writer,
        "const {name}: [{}; {}] = [",
        type_name::<T>(),
        slice.len()
    )?;

    for data in slice.iter() {
        write!(writer, "{data}, ")?;
    }

    writeln!(writer, "];")?;

    Ok(())
}
