//! an executable for embedding graph
//! 
//! example usage:
//! 
//! Hope mode for embedding with Adamic Adar approximation using approximation with a target rank of 1000 and 3 iterations
//! in the range approximations:  
//! embedder --csv "p2p-Gnutella09.txt" hope  --approx "ADA" rank --targetrank 1000 --nbiter 3
//! 
//! with precision target:  
//! embedder --csv "p2p-Gnutella09.txt" hope  --approx "ADA" precision --epsil 0.2 --maxrank 1000 --blockiter 3
//! 
//! Sketching embedding with 3 hop neighbourhood, weight decay factor of 0.1 at each hop, dimension 500 :
//! 
//! embedder --csv "p2p-Gnutella09.txt" sketching --decay 0.1  --dim 500 --nbiter 3 
//! 
//!The sketching mode can construct a symetric embedding by passing the -s flag
//! 
//! 
//! Embedding for estimation of AUC with link prediction 
//!     It suffices to add the command : **validation --npass nbpass --skip fraction**
//!     with nbpass is the number of step asked for in the validation and skip is the fraction of edges kept out of the train dataset.  
//!     example : embedder --csv "p2p-Gnutella09.txt" sketching --decay 0.1  --dim 500 --nbiter 3 validation --npass 10 --skip 0.1
//! 
//!  hope or nodesketch are differents algorithms for embedding see related docs
//!  for hope algorithms different modes of approximations are possible : KATZ, RPR (rooted page rank), ADA (adamic adar)
//!  




use anyhow::{anyhow};
use clap::{Arg, ArgMatches, Command, arg};

use graphite::prelude::*;
use sprs::{TriMatI};


static DATADIR : &str = &"/home/jpboth/Data/Graphs";


fn parse_sketching(matches : &ArgMatches) -> Result<NodeSketchParams, anyhow::Error> {
    log::debug!("in parse_sketching");
    // get embedding dimension
    let dimension = match matches.value_of("dim") {
        Some(str) => {
            let res = str.parse::<usize>();
            if res.is_ok() {
                res.unwrap()
            }
            else {
                return Err(anyhow!("error parsing dim"));
            }
        },
        _   => { return Err(anyhow!("error parsing dim")); },
    }; // end match

    // get decay
    let decay = match matches.value_of("decay") {
        Some(str) => {
            str.parse::<f64>().unwrap()
        },
        _   => { return Err(anyhow!("error parsing decay")); },
    }; // end match 

    // get nbiter
    let nb_iter = match matches.value_of("nbiter") {
        Some(str) => {
            let res = str.parse::<usize>();
            if res.is_ok() {
                res.unwrap()
            }
            else {
                return Err(anyhow!("error parsing decay"));
            }
        },
        _   => { return Err(anyhow!("error parsing decay")); },
    }; // end match
    //
    let sketch_params = NodeSketchParams{sketch_size: dimension, decay, nb_iter, parallel : true};
    return Ok(sketch_params);
} // end of parse_sketching



fn parse_hope_args(matches : &ArgMatches)  -> Result<HopeParams, anyhow::Error> {
    log::debug!("in parse_hope");
    // first get mode Katz or Rooted Page Rank
    let mut epsil : f64 = 0.;
    let mut maxrank : usize = 0;
    let mut blockiter = 0;
    let mut decay = 1.;
    // get approximation mode
    let hope_mode = match matches.value_of("proximity") {
        Some("KATZ") => {  HopeMode::KATZ
                        },
        Some("RPR")  => {   HopeMode::RPR
                        },
        Some("ADA")  => { HopeMode::ADA},
        _            => {
                            log::error!("did not get proximity used : ADA,KATZ or RPR");
                            std::process::exit(1);
                        },
    };
                      
    match matches.subcommand() {

        Some(("precision", sub_m)) =>  {
            if let Some(str) = sub_m.value_of("epsil") {
                let res = str.parse::<f64>();
                match res {
                    Ok(val) => { epsil = val;},
                    _            => { return Err(anyhow!("could not parse Hope epsil"));},
                }         
            } // end of epsil
 
            // get decay
            if let Some(str)  = sub_m.value_of("decay")  { 
                let res = str.parse::<f64>();
                match res {
                    Ok(val) => { decay = val},
                    _       => { return Err(anyhow!("could not parse Hope decay"));},
                } 
            };  // end of decay match   
    
            // get maxrank
            if let Some(str) = sub_m.value_of("maxrank") {
                let res = str.parse::<usize>();
                match res {
                    Ok(val) => { maxrank = val;},
                    _              => { return Err(anyhow!("could not parse Hope maxrank")); },
                }
            }

            // get blockiter
            if let Some(str) = sub_m.value_of("blockiter") {
                let res = str.parse::<usize>();
                match res {
                    Ok(val) => { blockiter = val;},
                    _              => { return Err(anyhow!("could not parse Hope blockiter"));},
                }        
            }
            //
            let range = RangeApproxMode::EPSIL(RangePrecision::new(epsil, blockiter, maxrank));
            let params = HopeParams::new(hope_mode, range, decay);
            return Ok(params);
        },  // end decoding precision arg


        Some(("rank", sub_m)) => {
            if let Some(str) = sub_m.value_of("targetrank") {
                let res = str.parse::<usize>();
                match res {
                    Ok(val) => { maxrank = val;},
                    _              => { return Err(anyhow!("could not parse Hope maxrank"));},
                }
            } // end of target rank

            // get blockiter
            if let Some(str) = sub_m.value_of("nbiter") {
                let res = str.parse::<usize>();
                match res {
                    Ok(val) => { blockiter = val ; },
                    _              => {  return Err(anyhow!("could not parse Hope blockiter")); }
                }    
            }   
            //          
            let range = RangeApproxMode::RANK(RangeRank::new(maxrank, blockiter));
            let params = HopeParams::new(hope_mode, range, decay);
            return Ok(params);
        }, // end of decoding rank arg

        _  => {
            log::error!("could not decode hope argument, got neither precision nor rank subcommands");
            return Err(anyhow!("could not parse Hope parameters"));
        },

    }; // end match
} // end of parse_hope_args

//=======================================================================

struct EmbeddingParams {
    mode : EmbeddingMode,
    hope : Option<HopeParams>,
    sketching : Option<NodeSketchParams>,
} // end of struct EmbeddingParams


impl From<HopeParams> for EmbeddingParams {
    fn from(params : HopeParams) -> Self {
        EmbeddingParams{mode : EmbeddingMode::Hope, hope : Some(params), sketching:None}
    }
}

impl From<NodeSketchParams> for EmbeddingParams {
    fn from(params : NodeSketchParams) -> Self {
        EmbeddingParams{mode : EmbeddingMode::Nodesketch, hope : None, sketching: Some(params)}
    }
}

//=================================================================


struct ValidationCmd {
    validation_params : ValidationParams,
    embedding_params : EmbeddingParams,
} // end of struct ValidationCmd





fn parse_validation_cmd(matches : &ArgMatches) ->  Result<ValidationCmd, anyhow::Error> {
    //
    log::debug!("in parse_validation_parameters");
    // for now only link prediction is implemented
    let delete_proba : f64;
    let nbpass : usize;

    match matches.value_of("skip") {
        Some(str) =>  { 
                let res = str.parse::<f64>();
                match res {
                    Ok(val) => { delete_proba = val},
                    _       => { return Err(anyhow!("could not parse skip parameter"));
                                },
                } 
        } 
        _      => { return Err(anyhow!("could not parse decay"));}
    };  // end of skip match 

    match matches.value_of("nbpass") {
        Some(str) =>  { 
                let res = str.parse::<usize>();
                match res {
                    Ok(val) => { nbpass = val},
                    _       => { return Err(anyhow!("could not parse nbpass parameter"));
                                },
                } 
        } 
        _      => { return Err(anyhow!("could not parse decay"));}
    };  // end of skip match 
    // 
    let validation_params = ValidationParams::new(delete_proba, nbpass);
    //
    //
    match matches.subcommand() {
        Some(("hope", sub_m))       => {
                if let Ok(params) = parse_hope_args(sub_m) {
                    return Ok(ValidationCmd{validation_params, embedding_params : EmbeddingParams::from(params)});
                }
                else { 
                    log::error!("parse_hope_args failed");
                    return Err(anyhow!("parse_hope_args failed"));
                }
        },
        Some(("sketching" , sub_m)) => {
                if let Ok(params) = parse_sketching(sub_m) {
                    return Ok(ValidationCmd{validation_params, embedding_params : EmbeddingParams::from(params)});
                }
                else { 
                    log::error!("parse_hope_args failed");
                    return Err(anyhow!("parse_hope_args failed"));
                }
        },
           _                                    => {
                log::error!("did not find hope neither sketching commands");
        },
    }    //
    return Err(anyhow!("parse_validation_cmd failed"));
}  // end of parse_validation_cmd




fn parse_embedding_cmd(matches : &ArgMatches) ->  Result<EmbeddingParams, anyhow::Error> {
    match matches.subcommand() {
        Some(("hope", sub_m))       => {
                if let Ok(params) = parse_hope_args(sub_m) {
                    return Ok(EmbeddingParams::from(params));
                }
                else { 
                    log::error!("parse_hope_args failed");
                    return Err(anyhow!("parse_hope_args failed"));
                }
        },
        Some(("sketching" , sub_m)) => {
                if let Ok(params) = parse_sketching(sub_m) {
                    return Ok(EmbeddingParams::from(params));
                }
                else { 
                    log::error!("parse_hope_args failed");
                    return Err(anyhow!("parse_hope_args failed"));
                }
        },
           _                                    => {
                log::error!("did not find hope neither sketching commands");
                return Err(anyhow!("parse_hope_args failed")); 
        },
    }
}  // parse_embedding_cmd


pub fn main() {
    //
    let _ = env_logger::builder().is_test(true).try_init();
    log::info!("logger initialized"); 

 

    // the hope embedding command
    let hope_cmd = Command::new("hope")
    .subcommand_required(false)
    .arg_required_else_help(true)
    .arg(Arg::new("proximity")
        .long("approx")
        .required(true)
        .takes_value(true)
        .help("specify KATZ or RPR"))
    .subcommand(Command::new("precision")
        .arg_required_else_help(true)
        .args(&[
            arg!(--decay <decay> "decay factor at each hop"),
            arg!(--maxrank <maxrank> "maximum rank expected"),
            arg!(--blockiter <blockiter> "integer between 2 and 5"),
            arg!(-e --epsil <epsil> "precision between 0. and 1."),
        ]))
    .subcommand(Command::new("rank")
        .arg_required_else_help(true)
        .args(&[
            arg!(--decay <decay> "decay factor at each hop"),
            arg!(--targetrank <targetrank> "expected rank"),
            arg!(--nbiter <nbiter> "integer between 2 and 5"),
        ])          
    );

    // the sketch embedding command
    let sketch_cmd = Command::new("sketching")
        .arg_required_else_help(true)
        .args(&[
            arg!(-d --dim <dim> "the embedding dimension"),
            arg!(--decay <decay> "decay coefficient"),
            arg!(--nbiter <nbiter> "number of loops around a node"),
        ])
        .arg(Arg::new("symetry")
            .short('s')
            .help(" -s for a symetric embedding, default is assymetric")
    );

    // validation must have one embedding subcommand
    let validation_cmd= Command::new("validation")
        .subcommand_required(true)
        .args(&[
            arg!(--nbpass <nbpass> "number of passes of validation"),
            arg!(--skip <fraction> "fraction of edges to skip in training set"),
            ])
        .subcommand(hope_cmd.clone())
        .subcommand(sketch_cmd.clone());

    // the embedding command does just the embedding
    let embedding_command = Command::new("embedding")
        .subcommand_required(true)
        .subcommand(hope_cmd.clone())
        .subcommand(sketch_cmd.clone());
    //
    let matches = Command::new("embedder")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(Arg::new("csvfile")
            .long("csv")    
            .takes_value(true)
            .required(true)
            .help("expecting a csv file"))
        .subcommand(embedding_command)
        .subcommand(validation_cmd)
    .get_matches();
    // decode args

    let mut fname = String::from("");
    if matches.is_present("csvfile") {
        let csv_file = matches.value_of("csvfile").ok_or("").unwrap().parse::<String>().unwrap();
        if csv_file == "" {
            println!("parsing of request_dir failed");
            std::process::exit(1);
        }
        else {
            log::info!("input file : {:?}", csv_file.clone());
            fname = csv_file.clone();
        }
    };

    let mut hope_params : Option<HopeParams> = None;
    let mut embedding_parameters : Option<EmbeddingParams> = None;
    let mut sketching_params : Option<NodeSketchParams> = None;
    let mut validation_params : Option<ValidationParams> = None;
    //
    match matches.subcommand() {
        Some(("validation", sub_m)) => {
            log::debug!("got validation command");
            let res = parse_validation_cmd(sub_m);
            match res {
                Ok(cmd) =>  { 
                                                validation_params = Some(cmd.validation_params);
                                                embedding_parameters = Some(cmd.embedding_params);
                                         },
                _                     => {  },
            }
        },

        Some(("embedding", sub_m )) => {
            log::debug!("got embedding command");
            let res = parse_embedding_cmd(sub_m);
            match res {
                Ok(params) => { embedding_parameters = Some(params); },
                _                     => {  },
            }
        }

        _  => {
            log::error!("expected subcommand hope or nodesketch");
            std::process::exit(1);
        }
    }  // end match subcommand

    if let Some(validation_m) = matches.subcommand_matches("validation") {
        log::debug!("subcommand_matches got subcommand match");
        let res = parse_validation_cmd(validation_m);        
        match res {
            Ok(cmd) => { validation_params = Some(cmd.validation_params); },
            _                          => {  },
        }
    }  // end if validation



    log::info!(" parsing of commands succeeded"); 
    //
    let path = std::path::Path::new(crate::DATADIR).join(fname.clone().as_str());
    log::info!("\n\n  loading file {:?}", path);
    let res = csv_to_trimat_delimiters::<f64>(&path, true);
    if res.is_err() {
        log::error!("error : {:?}", res.as_ref().err());
        log::error!("embedder failed in csv_to_trimat, reading {:?}", &path);
        std::process::exit(1);
    }
    let (trimat, node_index) = res.unwrap();
    //
    // we have our graph in trimat format
    //
    if hope_params.is_some() {
        log::info!("embedding mode : Hope");
        // now we allocate an embedder (sthing that implement the Embedder trait)
        if validation_params.is_none() {
            // we do the embedding
            let mut hope = Hope::new(hope_params.unwrap(), trimat); 
            let embedding = Embedding::new(node_index, &mut hope);
            if embedding.is_err() {
                log::error!("hope embedding failed, error : {:?}", embedding.as_ref().err());
                std::process::exit(1);
            };
            let _embed_res = embedding.unwrap();
            // should dump somewhere
        }
        else {
            let params = validation_params.unwrap();
            // have to run validation simulations
            log::info!("doing validaton runs for hope embedding");
            // construction of the function necessay for AUC iterations
            let f = | trimat : TriMatI<f64, usize> | -> EmbeddedAsym<f64> {
                let mut hope = Hope::new(hope_params.unwrap(), trimat); 
                let res = hope.embed();
                res.unwrap()
            };
            estimate_auc(&trimat.to_csr(), params.get_nbpass(), params.get_delete_fraction(), false, &f);
        }
    }  // end case Hope
    else if sketching_params.is_some() {
        log::info!("embedding mode : Sketching");
        if validation_params.is_none() {
            log::debug!("running embedding without validation");
            // now we allocate an embedder (sthing that implement the Embedder trait)
            let mut nodesketch = NodeSketchAsym::new(sketching_params.unwrap(), trimat);
            let embedding = Embedding::new(node_index, &mut nodesketch);
            if embedding.is_err() {
                log::error!("nodesketch embedding failed error : {:?}", embedding.as_ref().err());
                std::process::exit(1);
            };
            let _embed_res = embedding.unwrap();
        } // end case no validation
        else {
            let params = validation_params.unwrap();
            // have to run validation simulations
            log::info!("doing validaton runs for nodesketch embedding");
            // construction of the function necessay for AUC iterations            
            let f = | trimat : TriMatI<f64, usize> | -> EmbeddedAsym<usize> {
                let mut nodesketch = NodeSketchAsym::new(sketching_params.unwrap(), trimat);
                let res = nodesketch.embed();
                res.unwrap()
            };
            estimate_auc(&trimat.to_csr(), params.get_nbpass(), params.get_delete_fraction(), false, &f);
        }
    }  // end case sketching_params
    // 
    //    
}  // end fo main