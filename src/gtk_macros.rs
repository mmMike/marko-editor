#[macro_export]
macro_rules! builder_get {
    ($builder:ident($name:tt)) => {
        $builder.get_object($name).expect(&format!("Builder couldn't get {}", $name))
    };
}

#[macro_export]
macro_rules! connect {
    ($this:ident.$fct:ident($($arg:expr),*)) => {{
        let s = $this.clone();
        move |_| s.$fct($($arg,)*)
    }};
    ($this:ident.$a:ident.$fct:ident($($arg:expr),*)) => {{
        let s = $this.$a.clone();
        move |_| s.$fct($($arg,)*)
    }};
    ($this:ident.$a:ident.$b:ident.$fct:ident($($arg:expr),*)) => {{
        let s = $this.$a.$b.clone();
        move |_| s.$fct($($arg,)*)
    }};
}

#[macro_export]
macro_rules! connect_action_plain {
    ($this:ident.$fct:ident($($arg:expr),*)) => {{
        let s = $this.clone();
        move |_,_| s.$fct($($arg,)*)
    }};
    ($this:ident.$a:ident.$fct:ident($($arg:expr),*)) => {{
        let s = $this.$a.clone();
        move |_,_| s.$fct($($arg,)*)
    }};
}

#[macro_export]
macro_rules! connect_fwd1 {
    ($this:ident.$fct:ident()) => {{
        let s = $this.clone();
        move |a| s.$fct(a)
    }};
    ($this:ident.$a:ident.$fct:ident()) => {{
        let s = $this.$a.clone();
        move |a| s.$fct(a)
    }};
    ($this:ident.$a:ident.$b:ident.$fct:ident()) => {{
        let s = $this.$a.$b.clone();
        move |a| s.$fct(a)
    }};
}
